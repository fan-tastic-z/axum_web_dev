use crate::web::{Error, Result};
use futures::Future;
use lib_core::ctx::Ctx;
use lib_core::model::ModelManager;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::marker::PhantomData;
use std::pin::Pin;

// region:    --- RpcRouter

pub struct RpcRouter {
	pub(self) rpc_handlers: Vec<Box<dyn RpcRouteTrait>>,
}

impl RpcRouter {
	pub fn new() -> Self {
		Self {
			rpc_handlers: Vec::new(),
		}
	}

	pub fn add(mut self, erased_route: Box<dyn RpcRouteTrait>) -> Self {
		self.rpc_handlers.push(erased_route);
		self
	}

	pub fn append(mut self, mut other_router: RpcRouter) -> Self {
		self.rpc_handlers.append(&mut other_router.rpc_handlers);
		self
	}

	pub async fn call(
		&self,
		method: &str,
		ctx: Ctx,
		mm: ModelManager,
		params: Option<Value>,
	) -> Result<Value> {
		// Loop through all routes and call the matching one.
		for route in self.rpc_handlers.iter() {
			if route.is_route_for(method) {
				return route.call(ctx, mm, params).await;
			}
		}
		// If nothing match, return error.
		Err(Error::RpcMethodUnknow(method.to_string()))
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
                router = router.add($fn_name.into_boxed_rpc_route(stringify!($fn_name)));
            )+
            router
        }
    };
}

// endregion: --- RpcRouter

// region:    --- RpcHandler
type PinFutureValue = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

pub trait RpcHandler<T, R>: Clone {
	/// The type of future calling this handler returns.
	type Future: Future<Output = Result<Value>> + Send + 'static;

	/// Call the handler with the given request.
	fn call(self, ctx: Ctx, mm: ModelManager, params: Option<Value>)
		-> Self::Future;

	fn into_rpc_route(self, name: &'static str) -> RpcRoute<Self, T, R> {
		RpcRoute::new(self, name)
	}

	fn into_boxed_rpc_route(self, name: &'static str) -> Box<RpcRoute<Self, T, R>> {
		Box::new(RpcRoute::new(self, name))
	}
}

/// `IntoHandlerParams` enables converting an `Option<Value>` into
/// the required type for RPC handler parameters.
/// The default implementation below will fail if the value is `None`.
/// For custom behavior, users can implement their own `into_handler_params`
/// method.
pub trait IntoHandlerParams: DeserializeOwned + Send {
	fn into_handler_params(value: Option<Value>) -> Result<Self> {
		match value {
			Some(value) => Ok(serde_json::from_value(value)?),
			None => Err(Error::RpcIntoParamsMissing),
		}
	}
}

/// Marker trait with a blanket implementation that
pub trait IntoDefaultHandlerParams: DeserializeOwned + Send + Default {}

impl<P> IntoHandlerParams for P
where
	P: IntoDefaultHandlerParams,
{
	fn into_handler_params(value: Option<Value>) -> Result<Self> {
		match value {
			Some(value) => Ok(serde_json::from_value(value)?),
			None => Ok(Self::default()),
		}
	}
}

impl<F, Fut, R> RpcHandler<(), R> for F
where
	F: FnOnce(Ctx, ModelManager) -> Fut + Clone + Send + 'static,
	R: Serialize,
	Fut: Future<Output = Result<R>> + Send,
{
	type Future = PinFutureValue;

	fn call(
		self,
		ctx: Ctx,
		mm: ModelManager,
		params: Option<Value>,
	) -> Self::Future {
		Box::pin(async move {
			let result = self(ctx, mm).await?;
			Ok(serde_json::to_value(result)?)
		})
	}
}

impl<F, Fut, T, R> RpcHandler<(T,), R> for F
where
	T: IntoHandlerParams,
	F: FnOnce(Ctx, ModelManager, T) -> Fut + Clone + Send + 'static,
	R: Serialize,
	Fut: Future<Output = Result<R>> + Send,
{
	type Future = PinFutureValue;

	fn call(
		self,
		ctx: Ctx,
		mm: ModelManager,
		params_value: Option<Value>,
	) -> Self::Future {
		Box::pin(async move {
			// NOTE: For now, we require the params not to be None
			//       when the handler takes the params argument.
			// TODO: Needs to find a way to support Option<T> as handler params.
			let param = T::into_handler_params(params_value)?;

			let result = self(ctx, mm, param).await?;
			Ok(serde_json::to_value(result)?)
		})
	}
}

// endregion: --- RpcHandler

// region:    --- RpcHandlerRoute

// Note: This is the Wrapper also used as a Route (with the .name)
#[derive(Clone)]
pub struct RpcRoute<H, T, R> {
	name: &'static str,
	handler: H,
	_marker: PhantomData<(T, R)>,
}

// Constructor Impl
impl<H, T, R> RpcRoute<H, T, R> {
	pub fn new(handler: H, name: &'static str) -> Self {
		Self {
			name,
			handler,
			_marker: PhantomData,
		}
	}
}

// Caller Impl
impl<H, T, R> RpcRoute<H, T, R>
where
	H: RpcHandler<T, R> + Send + Sync + 'static,
	T: Send + Sync,
{
	pub fn call(
		&self,
		ctx: Ctx,
		mm: ModelManager,
		params: Option<Value>,
	) -> H::Future {
		let handler = self.handler.clone();
		RpcHandler::call(handler, ctx, mm, params)
	}
}

// Note: To make as HandlerRoute trait object.
pub trait RpcRouteTrait: Send + Sync {
	fn is_route_for(&self, method: &str) -> bool;

	fn call(
		&self,
		ctx: Ctx,
		mm: ModelManager,
		params: Option<Value>,
	) -> PinFutureValue;
}

impl<H, T, R> RpcRouteTrait for RpcRoute<H, T, R>
where
	H: RpcHandler<T, R> + Clone + Send + Sync + 'static,
	T: Send + Sync,
	R: Send + Sync,
{
	fn is_route_for(&self, method: &str) -> bool {
		method == self.name
	}

	fn call(
		&self,
		ctx: Ctx,
		mm: ModelManager,
		params: Option<Value>,
	) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> {
		Box::pin(self.call(ctx, mm, params))
	}
}

// endregion: --- RpcHandlerRoute
