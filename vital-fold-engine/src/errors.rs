use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

/// Unified error type for the application.
/// All errors are mapped to AppError variants for consistent handling.
#[derive(Debug, Error)]
pub enum AppError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Authentication/authorization failed
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Invalid request (e.g., malformed input)
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Resource conflict (e.g., a run is already in progress)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Internal server error
    #[error("Internal server error")]
    Internal(String),
}

/// JSON error response sent to clients
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl AppError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error message to return to the client
    pub fn client_message(&self) -> String {
        match self {
            AppError::Database(_) => "Database error occurred".to_string(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::Unauthorized(msg) => msg.clone(),
            AppError::BadRequest(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            AppError::Internal(_) => "Internal server error occurred".to_string(),
        }
    }
}

/// Implement ResponseError for AppError to make it work with Actix
impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        self.status_code()
    }

    fn error_response(&self) -> HttpResponse {
        // Log 500 errors before returning response
        if matches!(self, AppError::Database(_) | AppError::Internal(_)) {
            tracing::error!("Internal error: {}", self);
        }

        HttpResponse::build(self.status_code()).json(ErrorResponse {
            error: self.client_message(),
        })
    }
}

/// Convert sqlx errors to AppError
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("sqlx error: {}", err);
        AppError::Database(err.to_string())
    }
}

/// Convert bcrypt errors to AppError
impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        tracing::error!("bcrypt error: {}", err);
        AppError::Internal(err.to_string())
    }
}

/// Convert jsonwebtoken errors to AppError
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        tracing::error!("jsonwebtoken error: {}", err);
        AppError::Unauthorized("Invalid or expired token".to_string())
    }
}

/// Convert anyhow errors to AppError
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("anyhow error: {}", err);
        AppError::Internal(err.to_string())
    }
}
