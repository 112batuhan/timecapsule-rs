pub mod authentication;
pub mod emails;

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand_chacha::ChaCha8Rng;
use rand_core::{OsRng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::queries::{Db, DbError};
use crate::resend::{self, ExternalRequest};

#[derive(Clone)]
pub struct CurrentUser(i64);

impl CurrentUser {
    // For readability. I don't like using .0 for single element tuples
    pub fn get_user_id(&self) -> i64 {
        self.0
    }
}

pub struct SharedState {
    database: Db,
    random: Arc<Mutex<ChaCha8Rng>>,
    external_request: ExternalRequest,
}

impl SharedState {
    pub async fn new() -> Result<Arc<Self>, ApiError> {
        let database = Db::new().await?;

        let random = ChaCha8Rng::seed_from_u64(OsRng.next_u64());
        let random = Arc::new(Mutex::new(random));

        let external_request = ExternalRequest::new();

        Ok(Arc::new(SharedState {
            database,
            random,
            external_request,
        }))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorMessage {
    error_type: String,
    message: String,
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("General internal error.")]
    General,
    // Can't use #[from] here because missing std::error implementation in hash library
    #[error("Error during hashing.")]
    Hash,
    #[error("Database error: {0}")]
    Database(#[from] DbError),
    #[error("Wrong password.")]
    WrongPassword,
    #[error("Missing token from the client request.")]
    MissingSessionTokenInClientRequest,
    #[error("Bad E-Mail.")]
    BadEmail,
    #[error("Bad Password.")]
    BadPassword,
    #[error("Date parse error: {0}")]
    BadDate(#[from] chrono::ParseError),
    // later improve this. reqwest errors are way to convoluted.
    #[error("Error while Email sending: {0}")]
    EmailSend(#[from] resend::ResendError),
    #[error("User tried to send a mail that doesn't belong to them.")]
    UnauthorizedEmail,
    #[error("Session token processing error")]
    TokenProcessing,
    #[error("Trying to manipulate a hidden e-mail")]
    HiddenEmail,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = self.to_error_json();
        let status_code = self.to_status_code();
        (status_code, body).into_response()
    }
}

impl ApiError {
    fn to_status_code(&self) -> StatusCode {
        match self {
            ApiError::General => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Hash => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::WrongPassword => StatusCode::UNAUTHORIZED,
            ApiError::Database(database_error) => match database_error {
                DbError::UniqueConstraintViolation => StatusCode::CONFLICT,
                DbError::PGDatabase(_) => StatusCode::INTERNAL_SERVER_ERROR,
                DbError::EmptyQuery => StatusCode::NO_CONTENT,
                DbError::MissingSessionTokenInDatabase => StatusCode::UNAUTHORIZED,
                DbError::RedisDatabase(_) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            ApiError::MissingSessionTokenInClientRequest => StatusCode::UNAUTHORIZED,
            ApiError::BadEmail => StatusCode::BAD_REQUEST,
            ApiError::BadDate(_) => StatusCode::BAD_REQUEST,
            // later improve this.
            ApiError::EmailSend(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::UnauthorizedEmail => StatusCode::FORBIDDEN,
            ApiError::BadPassword => StatusCode::BAD_REQUEST,
            ApiError::TokenProcessing => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::HiddenEmail => StatusCode::FORBIDDEN,
        }
    }

    fn to_error_json(&self) -> Json<ErrorMessage> {
        let error_type = match self {
            ApiError::General => "Unhandled_internal_error".to_string(),
            ApiError::Hash => "hash_error".to_string(),
            ApiError::WrongPassword => "password_mismatch".to_string(),
            ApiError::Database(database_error) => match database_error {
                DbError::UniqueConstraintViolation => "existing_user".to_string(),
                DbError::PGDatabase(_) => "unhandled_pg_database_error".to_string(),
                DbError::EmptyQuery => "empty_query_result".to_string(),
                DbError::MissingSessionTokenInDatabase => "missing_token_in_server".to_string(),
                DbError::RedisDatabase(_) => "unhandled_redis_database_error".to_string(),
            },
            ApiError::MissingSessionTokenInClientRequest => {
                "missing_token_in_client_request".to_string()
            }
            ApiError::BadEmail => "bad_email".to_string(),
            ApiError::BadDate(_) => "bad_date".to_string(),
            ApiError::EmailSend(_) => "email_send_error".to_string(),
            ApiError::UnauthorizedEmail => "unauthorized_email_send".to_string(),
            ApiError::BadPassword => "bad_password".to_string(),
            ApiError::TokenProcessing => "session_token_processing".to_string(),
            ApiError::HiddenEmail => "hidden_email_manipulation".to_string(),
        };

        let message = self.to_string();

        Json(ErrorMessage {
            error_type,
            message,
        })
    }
}
