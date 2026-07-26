use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Access denied: {0}")]
    Authorization(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Too many requests")]
    RateLimit,

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("External service error: {0}")]
    ExternalService(String),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponse {
    pub success: bool,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Authentication(_) => StatusCode::UNAUTHORIZED,
            Self::Authorization(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Database(err) => {
                tracing::error!("Database error occurred: {:?}", err);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::Internal(err) => {
                tracing::error!("Internal error occurred: {}", err);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            Self::WebSocket(_) => StatusCode::BAD_REQUEST,
            Self::ExternalService(err) => {
                tracing::error!("External service error occurred: {}", err);
                StatusCode::BAD_GATEWAY
            }
        }
    }

    pub fn error_message(&self) -> String {
        match self {
            Self::Database(_) => "A database error occurred. Please try again later.".to_string(),
            Self::Internal(_) => "An internal server error occurred.".to_string(),
            _ => self.to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = self.error_message();

        let body = Json(ApiErrorResponse {
            success: false,
            error: message,
            details: match &self {
                Self::Validation(detail_msg) => {
                    Some(serde_json::json!({ "validation": detail_msg }))
                }
                _ => None,
            },
        });

        (status, body).into_response()
    }
}

pub type Result<T, E = AppError> = std::result::Result<T, E>;
