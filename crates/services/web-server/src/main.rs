mod error;
mod log;
mod web;

use crate::web::{
	mw_auth::{mw_ctx_require, mw_ctx_resolve},
	mw_req_stamp::mw_req_stamp,
	mw_res_map::mw_reponse_map,
	routes_login, routes_static,
	rpc::{self, RpcState},
};

pub use self::error::{Error, Result};

use std::net::SocketAddr;

use axum::{middleware, Router};

use lib_core::{_dev_utils, model::ModelManager};
use tower_cookies::CookieManagerLayer;

use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.without_time()
		.with_target(false)
		.with_env_filter(EnvFilter::from_default_env())
		.init();

	// -- FOR DEV ONLY
	_dev_utils::init_dev().await;

	// Initialze ModelManager.
	let mm = ModelManager::new().await?;
	// -- Define Routes
	let rpc_state = RpcState { mm: mm.clone() };

	let routes_rpc =
		rpc::routes(rpc_state).route_layer(middleware::from_fn(mw_ctx_require));

	let routes_all = Router::new()
		.merge(routes_login::routes(mm.clone()))
		.nest("/api", routes_rpc)
		.layer(middleware::map_response(mw_reponse_map))
		.layer(middleware::from_fn_with_state(mm.clone(), mw_ctx_resolve))
		.layer(middleware::from_fn(mw_req_stamp))
		.layer(CookieManagerLayer::new())
		.fallback_service(routes_static::serve_dir());

	// region:    --- Start Server
	let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
	info!("{:<12} - {addr}\n", "LISTENING");
	axum::Server::bind(&addr)
		.serve(routes_all.into_make_service())
		.await
		.unwrap();
	// endregion: --- Start Server

	Ok(())
}
