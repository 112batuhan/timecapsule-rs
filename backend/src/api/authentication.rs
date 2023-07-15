use std::sync::Arc;

use axum::extract::State;
use axum::http::header::{self, COOKIE};
use axum::http::{HeaderMap, Request};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use email_address::*;
use lazy_static::lazy_static;
use pbkdf2::password_hash::rand_core::OsRng;
use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use pbkdf2::Pbkdf2;
use rand_chacha::ChaCha8Rng;
use rand_core::RngCore;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::{ApiError, CurrentUser, SharedState};
use crate::entities::users;
use crate::queries::DbError;

const PASSWORD_MIN_LENGTH: usize = 6;
const SESSION_TOKEN_KEY: &'static str = "timecapsule_session_token";
// Set the value to the "SameSite=strict" in servers, set to empty string in local.
// Don't forget to set it in prod environment :)
lazy_static! {
    static ref COOKIE_SAME_SITE_STRING: String =
        std::env::var("RESEND_EMAIL_ADDRESS").unwrap_or("".to_string());
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestUserBody {
    email: String,
    password: String,
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash_result = Pbkdf2.hash_password(password.as_bytes(), &salt);

    // Manual result handling because missing std::error implementation in hash library
    match hash_result {
        Ok(hashed_password) => Ok(hashed_password.to_string()),
        Err(_) => Err(ApiError::Hash),
    }
}

async fn generate_session_token(random: Arc<Mutex<ChaCha8Rng>>) -> String {
    let mut u128_pool = [0u8; 16];
    {
        // to drop the lock early
        let mut random = random.lock().await;
        random.fill_bytes(&mut u128_pool);
    }
    u128::from_le_bytes(u128_pool).to_string()
}

fn extract_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let cookie_option = headers.get(COOKIE);
    cookie_option.map_or_else(
        || Err(ApiError::MissingSessionTokenInClientRequest),
        |unparsed_cookies| {
            let cookie_string = unparsed_cookies.to_str().unwrap();
            Ok(get_cookie_with_key(cookie_string, SESSION_TOKEN_KEY)?)
        },
    )
}

pub async fn sign_up(
    State(state): State<Arc<SharedState>>,
    Json(body): Json<RequestUserBody>,
) -> Result<StatusCode, ApiError> {
    if !EmailAddress::is_valid(&body.email) {
        return Err(ApiError::BadEmail);
    }
    if body.password.len() < PASSWORD_MIN_LENGTH {
        return Err(ApiError::BadPassword);
    }
    let hashed_password = hash_password(&body.password)?;

    state
        .database
        .create_user(body.email, hashed_password)
        .await?;

    Ok(StatusCode::CREATED)
}

pub async fn login(
    State(state): State<Arc<SharedState>>,
    Json(body): Json<RequestUserBody>,
) -> Result<Response, ApiError> {
    let user = state.database.get_user_by_email(&body.email).await?;
    let parsed_hash = PasswordHash::new(&user.password).unwrap();
    let password_result = Pbkdf2.verify_password(body.password.as_bytes(), &parsed_hash);
    if let Err(err) = password_result {
        return match err {
            pbkdf2::password_hash::Error::Password => Err(ApiError::WrongPassword),
            _ => Err(ApiError::Hash),
        };
    };

    let session_check = state.database.get_session_with_id(user.id).await;
    match session_check {
        Err(DbError::MissingSessionTokenInDatabase) => {}
        Err(rest) => return Err(ApiError::from(rest)),
        Ok(token) => {
            state.database.delete_session(&token).await?;
        }
    }

    let session_token = generate_session_token(state.random.clone()).await;
    state
        .database
        .insert_session(user.id, &session_token)
        .await?;

    let cookie_value = format!(
        "{}={}; Max-Age=3600; {}",
        SESSION_TOKEN_KEY, session_token, *COOKIE_SAME_SITE_STRING
    );
    let mut response = Json(user).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        header::HeaderValue::from_str(&cookie_value).unwrap(),
    );
    *response.status_mut() = StatusCode::CREATED;

    Ok(response)
}

pub async fn auto_login(
    Extension(session): Extension<CurrentUser>,
    State(state): State<Arc<SharedState>>,
) -> Result<Json<users::Model>, ApiError> {
    Ok(Json(
        state.database.get_user_by_id(session.get_user_id()).await?,
    ))
}

pub async fn logout(
    headers: HeaderMap,
    State(state): State<Arc<SharedState>>,
) -> Result<(), ApiError> {
    let token = extract_token(&headers)?;
    state.database.delete_session(&token).await?;
    Ok(())
}

pub async fn check_session_token<T>(
    State(state): State<Arc<SharedState>>,
    mut request: Request<T>,
    next: axum::middleware::Next<T>,
) -> Result<Response, ApiError> {
    let token = extract_token(&request.headers())?;
    let user_session = state.database.get_session_with_token(&token).await?;
    request.extensions_mut().insert(CurrentUser(user_session));
    Ok(next.run(request).await)
}

fn get_cookie_with_key(cookies: &str, cookie_key: &str) -> Result<String, ApiError> {
    let cookie_value = cookies
        .split(";")
        .map(|cookie| cookie.trim())
        .find(|cookie| cookie.starts_with(cookie_key))
        .ok_or(ApiError::MissingSessionTokenInClientRequest)?;

    let cookie_value = cookie_value
        .get(cookie_key.len() + 1..cookie_value.len())
        .ok_or(ApiError::TokenProcessing)?;

    Ok(cookie_value.to_string())
}

#[test]
fn test_token_extractor() {
    let raw_token =
        "hi=123; timecapsule_session_token=5727209675184565786134592316200670339; by=456";
    let result = "5727209675184565786134592316200670339";
    let token = get_cookie_with_key(raw_token, "timecapsule_session_token").unwrap();
    assert_eq!(result, token)
}
