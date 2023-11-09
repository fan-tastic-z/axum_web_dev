use crate::web::rpc::router::{IntoDefaultParams, IntoParams};
use modql::filter::ListOptions;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

/// Params structure for any RPC Create call.
#[derive(Deserialize)]
pub struct ParamsForCreate<D> {
	pub data: D,
}

impl<D> IntoParams for ParamsForCreate<D> where D: DeserializeOwned + Send {}

/// Params structure for any RPC Update call.
#[derive(Deserialize)]
pub struct ParamsForUpdate<D> {
	pub id: i64,
	pub data: D,
}

impl<D> IntoParams for ParamsForUpdate<D> where D: DeserializeOwned + Send {}

/// Params structure for any RPC Update call.
#[derive(Deserialize)]
pub struct ParamsIded {
	pub id: i64,
}
impl IntoParams for ParamsIded {}

/// Params structure for any RPC List call.
#[derive(Deserialize, Default)]
pub struct ParamsList<F> {
	pub filter: Option<F>,
	pub list_options: Option<ListOptions>,
}

impl<D> IntoDefaultParams for ParamsList<D> where D: DeserializeOwned + Send + Default
{}

// region:    --- General Implementations
/// Implements `IntoParams` for any type that also implements `IntoParams`.
///
/// Note: Application code might prefer to avoid this blanket implementation
///       and opt for enabling it on a per-type basis instead. If that's the case,
///       simply remove this general implementation and provide specific
///       implementations for each type.
impl<D> IntoParams for Option<D>
where
	D: DeserializeOwned + Send,
	D: IntoParams,
{
	fn into_params(value: Option<Value>) -> crate::web::Result<Self> {
		let value = value.map(|v| serde_json::from_value(v)).transpose()?;
		Ok(value)
	}
}

/// This is the IntoParams implementation for serde_json Value.
///
/// Note: As above, this might not be a capability app code might want to
///       allow for rpc_handlers, prefering to have everything strongly type.
///       In this case, just remove this impelementation
impl IntoParams for Value {}
// endregion: --- General Implementations