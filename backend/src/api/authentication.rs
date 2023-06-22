use std::sync::Arc;

use axum::extract::State;
use axum::http::header::{self, COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use pbkdf2::password_hash::rand_core::OsRng;
use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use pbkdf2::Pbkdf2;
use rand_chacha::ChaCha8Rng;
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::{ApiError, CurrentUser, SharedState};
use crate::entities::users;

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

fn parse_token(header: &HeaderValue) -> String {
    header
        .to_str()
        .unwrap()
        .split("=")
        .skip(1)
        .next()
        .unwrap()
        .to_string()
}

fn extract_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let token_option = headers.get(COOKIE);
    token_option.map_or_else(
        || Err(ApiError::MissingSessionTokenInClientRequest),
        |unparsed_token| Ok(parse_token(unparsed_token)),
    )
}

pub async fn sign_up(
    State(state): State<Arc<SharedState>>,
    Json(body): Json<RequestUserBody>,
) -> Result<(), ApiError> {
    let hashed_password = hash_password(&body.password)?;

    state
        .database
        .create_user(body.email, hashed_password)
        .await?;

    Ok(())
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

    let session_token = generate_session_token(state.random.clone()).await;
    state
        .database
        .upsert_session(user.id, session_token.clone())
        .await?;

    let cookie_value = format!("session_token={}; Max-Age=3600", session_token);
    let mut response = Json(user).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        header::HeaderValue::from_str(&cookie_value).unwrap(),
    );

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
    let user_session = state.database.get_session(&token).await?;
    request.extensions_mut().insert(CurrentUser(user_session));
    Ok(next.run(request).await)
}
