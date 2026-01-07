use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use fido_types::ErrorResponse;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    TooManyRequests(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message, details) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "Not Found", Some(msg)),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad Request", Some(msg)),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "Unauthorized", Some(msg)),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "Forbidden", Some(msg)),
            ApiError::TooManyRequests(msg) => (
                StatusCode::TOO_MANY_REQUESTS,
                "Too Many Requests",
                Some(msg),
            ),
            ApiError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    Some("An unexpected error occurred".to_string()),
                )
            }
        };

        let error_response = ErrorResponse {
            error: message.to_string(),
            details,
        };

        (status, Json(error_response)).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(err: rusqlite::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}
