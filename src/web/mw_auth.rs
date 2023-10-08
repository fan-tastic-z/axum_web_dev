use async_trait::async_trait;
use axum::{
	extract::{FromRequestParts, State},
	http::{request::Parts, Request},
	middleware::Next,
	response::Response,
};
use serde::Serialize;
use tower_cookies::{Cookie, Cookies};
use tracing::debug;

use crate::{
	crypt::token::{validate_web_token, Token},
	ctx::Ctx,
	model::{
		user::{UserBmc, UserForAuth},
		ModelManager,
	},
	web::{set_token_cookie, Error, Result, AUTH_TOKEN},
};

#[allow(dead_code)] // For now, until we have the rpc.
pub async fn mw_ctx_require<B>(
	ctx: Result<Ctx>,
	req: Request<B>,
	next: Next<B>,
) -> Result<Response> {
	debug!("{:<12} - mw_ctx_require - {ctx:?}", "MIDDLEWARE");

	ctx?;

	Ok(next.run(req).await)
}

pub async fn mw_ctx_resolve<B>(
	mm: State<ModelManager>,
	cookies: Cookies,
	mut req: Request<B>,
	next: Next<B>,
) -> Result<Response> {
	debug!("{:<12} - mw_ctx_resolve", "MIDDLEWARE");

	let ctx_ext_result = _ctx_resolve(mm, &cookies).await;

	if ctx_ext_result.is_err()
		&& !matches!(ctx_ext_result, Err(CtxExtError::TokenNotInCookie))
	{
		cookies.remove(Cookie::named(AUTH_TOKEN))
	}

	// Store the ctx_ext_result in the request extension
	// (for Ctx extractor)
	req.extensions_mut().insert(ctx_ext_result);

	Ok(next.run(req).await)
}

// region:    --- Ctx Extractor
#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Ctx {
	type Rejection = Error;

	async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
		debug!("{:<12} - Ctx", "EXTRACTOR");

		parts
			.extensions
			.get::<CtxExtResult>()
			.ok_or(Error::CtxExt(CtxExtError::CtxNotInRequestExt))?
			.clone()
			.map_err(Error::CtxExt)
	}
}

async fn _ctx_resolve(mm: State<ModelManager>, cookies: &Cookies) -> CtxExtResult {
	// -- Get Token String
	let token = cookies
		.get(AUTH_TOKEN)
		.map(|c| c.value().to_string())
		.ok_or(CtxExtError::TokenNotInCookie)?;
	// -- Parse Token
	let token = token
		.parse::<Token>()
		.map_err(|_| CtxExtError::TokenWrongFormat)?;
	// -- Get UserForAuth
	let user: UserForAuth =
		UserBmc::first_by_username(&Ctx::root_ctx(), &mm, &token.ident)
			.await
			.map_err(|ex| CtxExtError::ModelAccessError(ex.to_string()))?
			.ok_or(CtxExtError::UserNotFound)?;
	// -- Validate Token
	validate_web_token(&token, &user.token_salt.to_string())
		.map_err(|_| CtxExtError::FailValidate)?;
	// -- Update Token
	set_token_cookie(cookies, &user.username, &user.token_salt.to_string())
		.map_err(|_| CtxExtError::CannotSetTokenCookie)?;
	// -- Create CtxExtResult
	Ctx::new(user.id).map_err(|ex| CtxExtError::CtxCreateFail(ex.to_string()))
}

// endregion: --- Ctx Extractor

// region:    --- Ctx Extractor Result/Error
type CtxExtResult = core::result::Result<Ctx, CtxExtError>;

#[derive(Clone, Serialize, Debug)]
pub enum CtxExtError {
	TokenNotInCookie,
	TokenWrongFormat,

	CtxNotInRequestExt,

	UserNotFound,
	ModelAccessError(String),
	FailValidate,
	CannotSetTokenCookie,

	CtxCreateFail(String),
}
// endregion: --- Ctx Extractor Result/Error
