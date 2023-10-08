use crate::crypt::pwd;
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
use tower_cookies::{Cookie, Cookies};
use tracing::debug;

pub fn routes(mm: ModelManager) -> Router {
    Router::new()
        .route("/api/login", post(api_login_handler))
        .with_state(mm)
}

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

    // FIXME: Implement real auth-token generation/signature.
    cookies.add(Cookie::new(web::AUTH_TOKEN, "user-1.exp.sign"));

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
