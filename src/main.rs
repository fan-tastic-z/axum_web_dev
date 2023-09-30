mod config;
mod ctx;
mod error;
mod log;
mod model;
mod web;
// #[cfg(test)] // Commented during early development.
pub mod _dev_utils;

use crate::{model::ModelManager, web::routes_static};

pub use self::error::{Error, Result};
pub use config::Config;

use std::net::SocketAddr;

use axum::{
    extract::{Path, Query},
    http::{Method, Uri},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use ctx::Ctx;
use serde::Deserialize;

use serde_json::json;
use tower_cookies::CookieManagerLayer;

use tracing::debug;
use tracing::info;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

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

    // Initialize ModelController.
    // let mc = ModelController::new().await?;

    // let routes_apis = web::routes_tickets::routes(mc.clone())
    //     .route_layer(middleware::from_fn(web::mw_auth::mw_require_auth));

    let routes_all = Router::new()
        .merge(routes_hello())
        // .merge(web::routes_login::routes())
        // .nest("/api", routes_apis)
        // .layer(middleware::map_response(main_response_mapper))
        // .layer(middleware::from_fn_with_state(
        //     mc.clone(),
        //     web::mw_auth::mw_ctx_resolver,
        // ))
        .layer(CookieManagerLayer::new())
        .fallback_service(routes_static::serve_dir());

    let addr = SocketAddr::from(([127, 0, 0, 1], 7000));

    info!("LISTING on {addr}\n");

    // region:    --- Start server
    axum::Server::bind(&addr)
        .serve(routes_all.into_make_service())
        .await
        .unwrap();
    // endregion: --- Start server

    Ok(())
}

// async fn main_response_mapper(
//     ctx: Option<Ctx>,
//     uri: Uri,
//     req_method: Method,
//     res: Response,
// ) -> Response {
//     debug!(" {:<12} - main_response_mapper", "RES_MAPPER");
//     let uuid = Uuid::new_v4();

//     // -- Get the eventual response error.
//     let service_error = res.extensions().get::<Error>();
//     // let client_status_error = service_error.map(|se| se.client_status_and_error());

//     // -- if client error, build the new response
//     let error_response = client_status_error
//         .as_ref()
//         .map(|(status_code, client_error)| {
//             let client_error_body = json!({
//                 "error": {
//                     "type": client_error.as_ref(),
//                     "req_uuid": uuid.to_string(),
//                 }
//             });
//             println!(" ->> client_error_body: {client_error_body}");

//             // Build the new response from the client_error_body
//             (*status_code, Json(client_error_body)).into_response()
//         });
//     // build and log the server log line
//     // let client_error = client_status_error.unzip().1;
//     // let _ = log_request(uuid, req_method, uri, ctx, service_error, client_error).await;

//     debug!("\n");
//     error_response.unwrap_or(res)
// }

// region:    --- Routes Hello

fn routes_hello() -> Router {
    Router::new()
        .route("/hello", get(handle_hello))
        .route("/hello2/:name", get(handler_hello2))
}

#[derive(Debug, Deserialize)]
struct HelloParams {
    name: Option<String>,
}

// e.g.,`/hello?name=fan-tastic`
async fn handle_hello(Query(params): Query<HelloParams>) -> impl IntoResponse {
    debug!(" {:<12} - handler_hello - {params:?}", "HANDLER");

    let name = params.name.as_deref().unwrap_or("World!");
    Html(format!("Hello <Strong>{name}</strong>"))
}
// e.g, `hello2/Mike`
async fn handler_hello2(Path(name): Path<String>) -> impl IntoResponse {
    debug!(" {:<12} - handler_hello2 - {name:?}", "HANDLER");
    Html(format!("Hello2 <strong>{name}</strong>"))
}
// endregion: ---  Routes Hello
