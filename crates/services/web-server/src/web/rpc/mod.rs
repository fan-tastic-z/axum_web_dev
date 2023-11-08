// region:    --- Modules

use crate::web::mw_auth::CtxW;
use axum::{
	extract::State,
	response::{IntoResponse, Response},
	routing::post,
	Json, Router,
};
use lib_core::model::ModelManager;
use modql::filter::ListOptions;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::web::rpc::infra::{
	IntoDefaultHandlerParams, IntoHandlerParams, RpcRouter,
};

mod infra;
mod project_rpc;
mod task_rpc;

// endregion: --- Modules

// region:    --- RPC Types

/// JSON-RPC Request Body
#[derive(Deserialize)]
struct RpcRequest {
	id: Option<Value>,
	method: String,
	params: Option<Value>,
}

#[derive(Deserialize)]
pub struct ParamsForCreate<D> {
	data: D,
}

impl<D> IntoHandlerParams for ParamsForCreate<D> where D: DeserializeOwned + Send {}

#[derive(Deserialize)]
pub struct ParamsForUpdate<D> {
	id: i64,
	data: D,
}

impl<D> IntoHandlerParams for ParamsForUpdate<D> where D: DeserializeOwned + Send {}

#[derive(Deserialize)]
pub struct ParamsIded {
	id: i64,
}

impl IntoHandlerParams for ParamsIded {}

#[derive(Deserialize, Default)]
pub struct ParamsList<F> {
	filter: Option<F>,
	list_options: Option<ListOptions>,
}

impl<D> IntoDefaultHandlerParams for ParamsList<D> where
	D: DeserializeOwned + Send + Default
{
}

// endregion: --- RPC Types

pub fn routes(mm: ModelManager) -> Router {
	// Build the combined RpcRouter.
	let mut rpc_router = RpcRouter::new()
		.append(task_rpc::rpc_router())
		.append(project_rpc::rpc_router());

	// Build the Axum States needed for this axum Router.
	let rpc_states = RpcStates(mm, Arc::new(rpc_router));

	// Build the Acum Router for '/rpc'
	Router::new()
		.route("/rpc", post(rpc_axum_handler))
		.with_state(rpc_states)
}

/// RPC basic information containing the id and method for additional logging purposes.
#[derive(Debug)]
pub struct RpcInfo {
	pub id: Option<Value>,
	pub method: String,
}

#[derive(Clone)]
struct RpcStates(ModelManager, Arc<RpcRouter>);

async fn rpc_axum_handler(
	State(RpcStates(mm, rpc_router)): State<RpcStates>,
	ctx: CtxW,
	Json(rpc_req): Json<RpcRequest>,
) -> Response {
	let ctx = ctx.0;

	// -- Create the RPC Info
	//    (will be set to the response.extensions)
	let rpc_info = RpcInfo {
		id: rpc_req.id.clone(),
		method: rpc_req.method.clone(),
	};
	// -- Exec Rpc Route
	let res = rpc_router
		.call(&rpc_info.method, ctx, mm, rpc_req.params)
		.await;

	// -- Build Rpc Success Response
	let res = res.map(|v| {
		let body_response = json!({
			"id": rpc_info.id,
			"result": v
		});
		Json(body_response)
	});

	// -- Create and Update Axum Response
	let mut res = res.into_response();
	res.extensions_mut().insert(rpc_info);

	res
}
