//! API error handling module for Fido server
//!
//! This module provides API error handling that integrates with SecureError
//! to ensure:
//! - Internal errors return generic messages to clients
//! - Database errors are logged but not exposed
//! - Stack traces and schema details are never leaked

use axum::response::{IntoResponse, Response};

use crate::security::errors::SecureError;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    TooManyRequests(String),
    /// Internal errors - details are logged but never exposed to clients
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Convert to SecureError for consistent, secure error handling
        let secure_error: SecureError = self.into();
        secure_error.into_response()
    }
}

impl From<ApiError> for SecureError {
    fn from(api_error: ApiError) -> Self {
        match api_error {
            ApiError::NotFound(msg) => SecureError::not_found(msg),
            ApiError::BadRequest(msg) => SecureError::bad_request(msg),
            ApiError::Unauthorized(msg) => SecureError::authentication(msg),
            ApiError::Forbidden(msg) => SecureError::authorization(msg),
            ApiError::TooManyRequests(msg) => SecureError::rate_limit(msg),
            // Internal errors: log details but return generic message to client
            ApiError::InternalError(msg) => SecureError::internal(msg),
        }
    }
}

impl From<SecureError> for ApiError {
    fn from(secure_error: SecureError) -> Self {
        use crate::security::errors::ErrorCode;
        
        match secure_error.code() {
            ErrorCode::NotFound => ApiError::NotFound("Resource not found".to_string()),
            ErrorCode::BadRequest => ApiError::BadRequest("Bad request".to_string()),
            ErrorCode::ValidationError => ApiError::BadRequest("Validation failed".to_string()),
            ErrorCode::AuthenticationError => ApiError::Unauthorized("Authentication required".to_string()),
            ErrorCode::AuthorizationError => ApiError::Forbidden("Access denied".to_string()),
            ErrorCode::RateLimitExceeded => ApiError::TooManyRequests("Rate limit exceeded".to_string()),
            ErrorCode::DatabaseError | ErrorCode::InternalError | ErrorCode::Conflict => {
                ApiError::InternalError("Internal server error".to_string())
            }
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        // Log the actual error details server-side, return generic message
        tracing::error!(
            error = %err,
            "Internal error from anyhow::Error"
        );
        ApiError::InternalError(err.to_string())
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(err: rusqlite::Error) -> Self {
        // Log the actual database error details server-side
        // NEVER expose database schema or query details to clients
        tracing::error!(
            error = %err,
            "Database error occurred"
        );
        ApiError::InternalError(format!("Database error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_internal_error_converts_to_secure_error() {
        // Create an internal error with sensitive details
        let error = ApiError::InternalError(
            "SQLITE_CONSTRAINT: users.email UNIQUE constraint failed at row 42".to_string(),
        );

        // Convert to SecureError
        let secure_error: SecureError = error.into();

        // Verify it's an internal error type
        assert_eq!(
            secure_error.code(),
            crate::security::errors::ErrorCode::InternalError
        );

        // The display should NOT contain sensitive details
        let display = format!("{}", secure_error);
        assert!(!display.contains("SQLITE"));
        assert!(!display.contains("UNIQUE"));
        assert!(!display.contains("constraint"));
        assert!(!display.contains("row 42"));
        assert!(!display.contains("users.email"));

        // Verify generic message IS in the display
        assert!(display.contains("Internal server error"));
    }

    #[test]
    fn test_database_error_hides_schema() {
        // Simulate a database error with schema details
        let db_error = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(19), // SQLITE_CONSTRAINT
            Some("UNIQUE constraint failed: users.username".to_string()),
        );
        let error: ApiError = db_error.into();

        // Convert to SecureError
        let secure_error: SecureError = error.into();

        // The display should NOT contain schema details
        let display = format!("{}", secure_error);
        assert!(!display.contains("users.username"));
        assert!(!display.contains("UNIQUE constraint"));

        // Verify generic message IS in the display
        assert!(display.contains("Internal server error"));
    }

    #[test]
    fn test_not_found_error() {
        let error = ApiError::NotFound("User not found".to_string());
        let secure_error: SecureError = error.into();

        assert_eq!(
            secure_error.code(),
            crate::security::errors::ErrorCode::NotFound
        );
        assert_eq!(
            secure_error.code().status_code(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn test_bad_request_error() {
        let error = ApiError::BadRequest("Invalid input format".to_string());
        let secure_error: SecureError = error.into();

        assert_eq!(
            secure_error.code(),
            crate::security::errors::ErrorCode::BadRequest
        );
        assert_eq!(
            secure_error.code().status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_unauthorized_error() {
        let error = ApiError::Unauthorized("Missing session token".to_string());
        let secure_error: SecureError = error.into();

        assert_eq!(
            secure_error.code(),
            crate::security::errors::ErrorCode::AuthenticationError
        );
        assert_eq!(
            secure_error.code().status_code(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn test_forbidden_error() {
        let error = ApiError::Forbidden("Admin access required".to_string());
        let secure_error: SecureError = error.into();

        assert_eq!(
            secure_error.code(),
            crate::security::errors::ErrorCode::AuthorizationError
        );
        assert_eq!(
            secure_error.code().status_code(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn test_too_many_requests_error() {
        let error = ApiError::TooManyRequests("Rate limit exceeded".to_string());
        let secure_error: SecureError = error.into();

        assert_eq!(
            secure_error.code(),
            crate::security::errors::ErrorCode::RateLimitExceeded
        );
        assert_eq!(
            secure_error.code().status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_secure_error_to_api_error_conversion() {
        // Test that SecureError converts back to ApiError correctly
        let secure_error = SecureError::internal("test error");
        let api_error: ApiError = secure_error.into();

        // Should be an InternalError
        match api_error {
            ApiError::InternalError(msg) => {
                assert_eq!(msg, "Internal server error");
            }
            _ => panic!("Expected InternalError"),
        }
    }

    #[test]
    fn test_secure_error_not_found_to_api_error() {
        let secure_error = SecureError::not_found("User");
        let api_error: ApiError = secure_error.into();

        match api_error {
            ApiError::NotFound(msg) => {
                assert_eq!(msg, "Resource not found");
            }
            _ => panic!("Expected NotFound"),
        }
    }

    #[test]
    fn test_secure_error_database_to_api_error() {
        let secure_error = SecureError::database("connection failed");
        let api_error: ApiError = secure_error.into();

        // Database errors should become InternalError
        match api_error {
            ApiError::InternalError(msg) => {
                assert_eq!(msg, "Internal server error");
            }
            _ => panic!("Expected InternalError for database error"),
        }
    }
}
