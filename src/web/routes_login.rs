use crate::crypt::pwd;
use crate::web::remove_token_cookie;
use crate::{
	crypt::EncryptContent,
	ctx::Ctx,
	model::{
		user::{UserBmc, UserForLogin},
		ModelManager,
	},
	web::{self, Error, Result},
};
use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_cookies::Cookies;
use tracing::debug;

pub fn routes(mm: ModelManager) -> Router {
	Router::new()
		.route("/api/login", post(api_login_handler))
		.route("/api/logoff", post(api_logoff_handler))
		.with_state(mm)
}

// region:    --- Login

async fn api_login_handler(
	mm: State<ModelManager>,
	cookies: Cookies,
	payload: Json<LoginPayload>,
) -> Result<Json<Value>> {
	debug!("{:<12} - api_login_handler", "HANDLER");

	let axum::Json(LoginPayload {
		username,
		pwd: pwd_clear,
	}) = payload;
	let root_ctx = Ctx::root_ctx();

	// -- Get the user.
	let user: UserForLogin = UserBmc::first_by_username(&root_ctx, &mm, &username)
		.await?
		.ok_or(Error::LoginFailUsernameNotFound)?;
	let user_id = user.id;

	// -- Validate the password.
	let Some(pwd) = user.pwd else {
		return Err(Error::LoginFailUserHasNotPwd { user_id });
	};

	pwd::validate_pwd(
		&EncryptContent {
			salt: user.pwd_salt.to_string(),
			content: pwd_clear.clone(),
		},
		&pwd,
	)
	.map_err(|_| Error::LoginFailPwdNotMatching { user_id })?;

	// Set web token
	web::set_token_cookie(&cookies, &user.username, &user.token_salt.to_string())?;
	// Create the success body.
	let body = Json(json!({
		"result": {
			"success": true
		}
	}));

	Ok(body)
}

#[derive(Debug, Deserialize)]
struct LoginPayload {
	username: String,
	pwd: String,
}

// endregion: --- Login

// region:    --- Logoff

async fn api_logoff_handler(
	cookies: Cookies,
	Json(payload): Json<LogoffPayload>,
) -> Result<Json<Value>> {
	debug!("{:<12} - api_logoff_handler", "HANDLER");
	let should_logoff = payload.logoff;

	if should_logoff {
		remove_token_cookie(&cookies)?;
	}

	// Create the success body.
	let body = Json(json!({
		"result": {
			"logged_off": should_logoff
		}
	}));

	Ok(body)
}

#[derive(Debug, Deserialize)]
struct LogoffPayload {
	logoff: bool,
}

// endregion: --- Logoff
