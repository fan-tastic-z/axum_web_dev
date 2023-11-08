use crate::web::rpc::RpcState;
use crate::web::{Error, Result};
use futures::Future;
use lib_core::ctx::Ctx;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;

// region:    --- RpcRouter

pub struct RpcRouter {
	pub route_by_name: HashMap<&'static str, Box<dyn RpcRouteTrait>>,
}

impl RpcRouter {
	pub fn new() -> Self {
		Self {
			route_by_name: HashMap::new(),
		}
	}

	pub fn add(mut self, erased_route: Box<dyn RpcRouteTrait>) -> Self {
		self.route_by_name.insert(erased_route.name(), erased_route);
		self
	}

	pub fn extend(mut self, mut other_router: RpcRouter) -> Self {
		self.route_by_name.extend(other_router.route_by_name);
		self
	}

	pub async fn call(
		&self,
		method: &str,
		ctx: Ctx,
		rpc_state: RpcState,
		params: Option<Value>,
	) -> Result<Value> {
		if let Some(route) = self.route_by_name.get(method) {
			route.call(ctx, rpc_state, params).await
		} else {
			Err(Error::RpcMethodUnknown(method.to_string()))
		}
	}
}

/// A simple macro to create a new RpcRouter
/// and add each rpc handler-compatible function along with their corresponding names.
///
/// e.g.,
///
/// ```
/// rpc_router!(
///   create_project,
///   list_projects,
///   update_project,
///   delete_project
/// );
/// ```
/// Is equivalent to:
/// ```
/// RpcRouter::new()
///     .add(create_project.into_boxed_rpc_route("create_project"))
///     .add(list_projects.into_boxed_rpc_route("list_projects"))
///     .add(update_project.into_boxed_rpc_route("update_project"))
///     .add(delete_project.into_boxed_rpc_route("delete_project"))
/// ```
///
#[macro_export]
macro_rules! rpc_router {
    ($($fn_name:ident),+ $(,)?) => {
        {
            let mut router = RpcRouter::new();
            $(
                router = router.add($fn_name.into_boxed_route(stringify!($fn_name)));
            )+
            router
        }
    };
}

// endregion: --- RpcRouter

// region:    --- RpcHandler

/// The `Handler` trait that will be implemented by rpc handler functions.
///
/// Key points:
/// - Rpc handler functions are asynchronous, thus returning a Future of Result<Value>.
/// - The call format is normalized to `ctx`, `mm`, and `params`, which represent the json-rpc's optional value.
/// - `into_boxed_route` is a convenient method for converting a RpcHandler into a Boxed RpcRoute,
///   allowing for dynamic dispatch by the Router.
/// - A `RpcHandler` will typically be implemented for static functions, as `FnOnce`,
///   enabling them to be cloned with none or negligible performance impact,
///   thus facilitating the use of RpcRoute dynamic dispatch.
pub trait RpcHandler<S, P, R>: Clone {
	/// The type of future calling this handler returns.
	type Future: Future<Output = Result<Value>> + Send + 'static;

	/// Call the handler.
	fn call(
		self,
		ctx: Ctx,
		rpc_state: RpcState,
		params: Option<Value>,
	) -> Self::Future;

	fn into_boxed_route(self, name: &'static str) -> Box<RpcRoute<Self, S, P, R>> {
		Box::new(RpcRoute::new(self, name))
	}
}

/// `IntoHandlerParams` allows for converting an `Option<Value>` into
/// the necessary type for RPC handler parameters.
/// The default implementation below will result in failure if the value is `None`.
/// For customized behavior, users can implement their own `into_handler_params`
/// method.
pub trait IntoParams: DeserializeOwned + Send {
	fn into_params(value: Option<Value>) -> Result<Self> {
		match value {
			Some(value) => Ok(serde_json::from_value(value)?),
			None => Err(Error::RpcIntoParamsMissing),
		}
	}
}

/// Marker trait with a blanket implementation that return T::default
/// if the `params: Option<Value>` is none.
pub trait IntoDefaultParams: DeserializeOwned + Send + Default {}

impl<P> IntoParams for P
where
	P: IntoDefaultParams,
{
	fn into_params(value: Option<Value>) -> Result<Self> {
		match value {
			Some(value) => Ok(serde_json::from_value(value)?),
			None => Ok(Self::default()),
		}
	}
}

type PinFutureValue = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

/// RpcHanlder implementation for `my_rpc_handler(ctx, mm) -> Result<Serialize> `
impl<F, Fut, S, R> RpcHandler<S, (), R> for F
where
	F: FnOnce(Ctx, S) -> Fut + Clone + Send + 'static,
	S: From<RpcState> + Send,
	R: Serialize,
	Fut: Future<Output = Result<R>> + Send,
{
	type Future = PinFutureValue;

	fn call(
		self,
		ctx: Ctx,
		rpc_state: RpcState,
		_params: Option<Value>,
	) -> Self::Future {
		Box::pin(async move {
			let result = self(ctx, rpc_state.into()).await?;
			Ok(serde_json::to_value(result)?)
		})
	}
}

/// RpcHandler implementation for `my_rpc_handler(ctx, rpc_state, IntoParams) -> Result<Serialize>`.
/// Note: The trait bounds `Clone + Send + 'static` apply to `F`,
///       and `Fut` has its own trait bounds defined afterwards.
impl<F, Fut, S, P, R> RpcHandler<S, (P,), R> for F
where
	F: FnOnce(Ctx, S, P) -> Fut + Clone + Send + 'static,
	S: From<RpcState> + Send,
	P: IntoParams,
	R: Serialize,
	Fut: Future<Output = Result<R>> + Send,
{
	type Future = PinFutureValue;

	fn call(
		self,
		ctx: Ctx,
		rpc_state: RpcState,
		params_value: Option<Value>,
	) -> Self::Future {
		Box::pin(async move {
			let param = P::into_params(params_value)?;

			let result = self(ctx, rpc_state.into(), param).await?;
			Ok(serde_json::to_value(result)?)
		})
	}
}

// endregion: --- RpcHandler

// region:    --- RpcHandlerRoute

/// `RpcRoute` is a wrapper for `RpcHandler` that contains:
/// - `handler` - the actual `RpcHandler` function.
/// - `name` - the corresponding JSON-RPC method name to which this handler responds.
///
/// `RpcRoute` implements `RpcRouteTrait` for type erasure, facilitating dynamic dispatch.
#[derive(Clone)]
pub struct RpcRoute<H, S, P, R> {
	name: &'static str,
	handler: H,
	_marker: PhantomData<(S, P, R)>,
}

// Constructor Impl
impl<H, S, P, R> RpcRoute<H, S, P, R> {
	pub fn new(handler: H, name: &'static str) -> Self {
		Self {
			name,
			handler,
			_marker: PhantomData,
		}
	}
}

// Caller Impl
impl<H, S, P, R> RpcRoute<H, S, P, R>
where
	H: RpcHandler<S, P, R> + Send + Sync + 'static,
{
	pub fn call(
		&self,
		ctx: Ctx,
		rpc_state: RpcState,
		params: Option<Value>,
	) -> H::Future {
		// Note: Since handler is a FnOnce,
		//       we can use it only once, so we clone it.
		let handler = self.handler.clone();
		RpcHandler::call(handler, ctx, rpc_state, params)
	}
}

/// `RpcRouteTrait` enables `RpcRoute` to become a trait object,
/// allowing for dynamic dispatch.
pub trait RpcRouteTrait: Send + Sync {
	fn name(&self) -> &'static str;

	fn call(
		&self,
		ctx: Ctx,
		rpc_state: RpcState,
		params: Option<Value>,
	) -> PinFutureValue;
}

/// `RpcRouteTrait` for `RpcRoute`.
/// Note: This enables `RpcRouter` to contain `rpc_handlers: Vec<Box<dyn RpcRouteTrait>>`
///       for dynamic dispatch.
impl<H, S, P, R> RpcRouteTrait for RpcRoute<H, S, P, R>
where
	H: RpcHandler<S, P, R> + Clone + Send + Sync + 'static,
	S: Send + Sync,
	P: Send + Sync,
	R: Send + Sync,
{
	fn name(&self) -> &'static str {
		self.name
	}

	fn call(
		&self,
		ctx: Ctx,
		rpc_state: RpcState,
		params: Option<Value>,
	) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> {
		Box::pin(self.call(ctx, rpc_state, params))
	}
}

// endregion: --- RpcHandlerRoute
