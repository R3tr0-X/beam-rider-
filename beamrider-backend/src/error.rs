use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

use crate::dto::PaymentRequiredResponse;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("payment required")]
    PaymentRequired(Box<PaymentRequiredResponse>),
    #[error("payment verification failed: {0}")]
    PaymentVerification(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("external service error: {0}")]
    External(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl AppError {
    pub const fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) | Self::Config(_) | Self::Crypto(_) => StatusCode::BAD_REQUEST,
            Self::PaymentRequired(_) => StatusCode::PAYMENT_REQUIRED,
            Self::PaymentVerification(_) => StatusCode::FORBIDDEN,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::External(_) | Self::Db(_) | Self::Migrate(_) | Self::Http(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    const fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "bad_request",
            Self::PaymentRequired(_) => "payment_required",
            Self::PaymentVerification(_) => "payment_verification_failed",
            Self::Conflict(_) => "conflict",
            Self::NotFound(_) => "not_found",
            Self::Config(_) => "configuration_error",
            Self::Crypto(_) => "crypto_error",
            Self::External(_) => "external_service_error",
            Self::Db(_) => "database_error",
            Self::Migrate(_) => "migration_error",
            Self::Http(_) => "http_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let Self::PaymentRequired(body) = self {
            return (StatusCode::PAYMENT_REQUIRED, Json(*body)).into_response();
        }

        let status = self.status_code();
        let body = ErrorBody {
            error: self.code(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}
