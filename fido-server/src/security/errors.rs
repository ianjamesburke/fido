//! Secure error handling module for Fido server
//!
//! This module provides secure error handling that:
//! - Returns generic error messages to clients for internal errors
//! - Logs detailed error information server-side
//! - Prevents information leakage through error messages
//! - Supports specific validation errors without internal details

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use fido_types::ErrorResponse;
use std::fmt;

/// Error codes for categorizing errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Database operation failed
    DatabaseError,
    /// Authentication failed
    AuthenticationError,
    /// Authorization failed (forbidden)
    AuthorizationError,
    /// Resource not found
    NotFound,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Internal server error (catch-all)
    InternalError,
    /// Bad request (malformed input)
    BadRequest,
}

impl ErrorCode {
    /// Get the HTTP status code for this error code
    pub fn status_code(&self) -> StatusCode {
        match self {
            ErrorCode::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::AuthenticationError => StatusCode::UNAUTHORIZED,
            ErrorCode::AuthorizationError => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    /// Get the safe public message for this error code
    pub fn public_message(&self) -> &'static str {
        match self {
            ErrorCode::DatabaseError => "Internal server error",
            ErrorCode::AuthenticationError => "Authentication required",
            ErrorCode::AuthorizationError => "Access denied",
            ErrorCode::NotFound => "Resource not found",
            ErrorCode::RateLimitExceeded => "Too many requests",
            ErrorCode::InternalError => "Internal server error",
            ErrorCode::BadRequest => "Bad request",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A secure error that separates internal details from public messages
///
/// This struct ensures that:
/// - Internal error details are logged server-side
/// - Only safe, generic messages are returned to clients
/// - Stack traces and database schema details are never exposed
#[derive(Debug)]
pub struct SecureError {
    /// The error code categorizing this error
    code: ErrorCode,
    /// Internal error details (for logging only, never exposed to clients)
    internal_details: String,
    /// Optional public details safe to show to clients (e.g., validation errors)
    public_details: Option<String>,
}

impl SecureError {
    /// Create a new SecureError with internal details
    ///
    /// The internal_details will be logged but never exposed to clients.
    pub fn new(code: ErrorCode, internal_details: impl Into<String>) -> Self {
        Self {
            code,
            internal_details: internal_details.into(),
            public_details: None,
        }
    }

    /// Create a SecureError with public details safe to show to clients
    ///
    /// Use this for validation errors where specific feedback is helpful.
    pub fn with_public_details(
        code: ErrorCode,
        internal_details: impl Into<String>,
        public_details: impl Into<String>,
    ) -> Self {
        Self {
            code,
            internal_details: internal_details.into(),
            public_details: Some(public_details.into()),
        }
    }

    /// Create a database error (always returns generic message to client)
    pub fn database(err: impl fmt::Display) -> Self {
        Self::new(ErrorCode::DatabaseError, format!("Database error: {}", err))
    }

    /// Create an authentication error
    pub fn authentication(reason: impl Into<String>) -> Self {
        Self::new(ErrorCode::AuthenticationError, reason)
    }

    /// Create an authorization error
    pub fn authorization(reason: impl Into<String>) -> Self {
        Self::new(ErrorCode::AuthorizationError, reason)
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        let resource = resource.into();
        Self::with_public_details(
            ErrorCode::NotFound,
            format!("Resource not found: {}", resource),
            format!("{} not found", resource),
        )
    }

    /// Create a rate limit error
    pub fn rate_limit(details: impl Into<String>) -> Self {
        Self::with_public_details(
            ErrorCode::RateLimitExceeded,
            details,
            "Rate limit exceeded. Please try again later.",
        )
    }

    /// Create an internal error (always returns generic message to client)
    pub fn internal(err: impl fmt::Display) -> Self {
        Self::new(ErrorCode::InternalError, format!("Internal error: {}", err))
    }

    /// Create a bad request error
    pub fn bad_request(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self::with_public_details(ErrorCode::BadRequest, &reason, reason.clone())
    }

    /// Get the error code
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// Log internal details server-side
    ///
    /// This method logs the full internal error details including any
    /// sensitive information that should not be exposed to clients.
    pub fn log_internal_details(&self) {
        match self.code {
            ErrorCode::DatabaseError | ErrorCode::InternalError => {
                tracing::error!(
                    error_code = %self.code,
                    internal_details = %self.internal_details,
                    "Secure error occurred"
                );
            }
            ErrorCode::AuthenticationError | ErrorCode::AuthorizationError => {
                tracing::warn!(
                    error_code = %self.code,
                    internal_details = %self.internal_details,
                    "Security error occurred"
                );
            }
            ErrorCode::RateLimitExceeded => {
                tracing::warn!(
                    error_code = %self.code,
                    internal_details = %self.internal_details,
                    "Rate limit exceeded"
                );
            }
            ErrorCode::BadRequest if self.internal_details.contains("authorization_pending") => {
                tracing::warn!(
                    error_code = %self.code,
                    internal_details = %self.internal_details,
                    "OAuth device flow still pending authorization"
                );
            }
            _ => {
                tracing::debug!(
                    error_code = %self.code,
                    internal_details = %self.internal_details,
                    "Client error occurred"
                );
            }
        }
    }

    /// Convert to an HTTP response
    ///
    /// This method:
    /// 1. Logs internal details server-side
    /// 2. Returns a safe response to the client without internal details
    pub fn to_response(&self) -> Response {
        // Always log internal details first
        self.log_internal_details();

        let status = self.code.status_code();
        let message = self.code.public_message();

        // Only include public_details if they exist and are safe
        let details = self.public_details.clone();

        let error_response = ErrorResponse {
            error: message.to_string(),
            details,
        };

        (status, Json(error_response)).into_response()
    }
}

impl IntoResponse for SecureError {
    fn into_response(self) -> Response {
        self.to_response()
    }
}

impl fmt::Display for SecureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display only shows the public message, not internal details
        write!(f, "{}", self.code.public_message())
    }
}

impl std::error::Error for SecureError {}

// Conversion from common error types

impl From<rusqlite::Error> for SecureError {
    fn from(err: rusqlite::Error) -> Self {
        // Never expose database error details to clients
        SecureError::database(err)
    }
}

impl From<anyhow::Error> for SecureError {
    fn from(err: anyhow::Error) -> Self {
        // Never expose internal error details to clients
        SecureError::internal(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_status_codes() {
        assert_eq!(
            ErrorCode::DatabaseError.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ErrorCode::AuthenticationError.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ErrorCode::AuthorizationError.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(ErrorCode::NotFound.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(
            ErrorCode::RateLimitExceeded.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            ErrorCode::InternalError.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(ErrorCode::BadRequest.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_code_public_messages() {
        // Database errors should return generic message
        assert_eq!(
            ErrorCode::DatabaseError.public_message(),
            "Internal server error"
        );
        // Internal errors should return generic message
        assert_eq!(
            ErrorCode::InternalError.public_message(),
            "Internal server error"
        );
    }

    #[test]
    fn test_secure_error_database_hides_details() {
        let err = SecureError::database("SQLITE_CONSTRAINT: users.email UNIQUE constraint failed");

        // The display should not contain internal details
        let display = format!("{}", err);
        assert!(!display.contains("SQLITE"));
        assert!(!display.contains("UNIQUE"));
        assert!(!display.contains("constraint"));
        assert_eq!(display, "Internal server error");

        // The error code should be correct
        assert_eq!(err.code(), ErrorCode::DatabaseError);
    }

    #[test]
    fn test_secure_error_internal_hides_details() {
        let err = SecureError::internal("Stack trace: at main.rs:42\n  at handler.rs:100");

        // The display should not contain stack trace
        let display = format!("{}", err);
        assert!(!display.contains("Stack trace"));
        assert!(!display.contains("main.rs"));
        assert!(!display.contains("handler.rs"));
        assert_eq!(display, "Internal server error");
    }

    #[test]
    fn test_secure_error_not_found_shows_resource() {
        let err = SecureError::not_found("User");

        assert_eq!(err.code(), ErrorCode::NotFound);
        assert!(err.public_details.is_some());
        assert!(err.public_details.as_ref().unwrap().contains("not found"));
    }

    #[test]
    fn test_secure_error_from_rusqlite() {
        // Simulate a rusqlite error
        let sqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let secure_err: SecureError = sqlite_err.into();

        // Should be converted to a database error with hidden details
        assert_eq!(secure_err.code(), ErrorCode::DatabaseError);
        let display = format!("{}", secure_err);
        assert_eq!(display, "Internal server error");
    }

    #[test]
    fn test_secure_error_authentication() {
        let err = SecureError::authentication("Invalid session token");

        assert_eq!(err.code(), ErrorCode::AuthenticationError);
        assert_eq!(err.code().status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_secure_error_authorization() {
        let err = SecureError::authorization("User is not admin");

        assert_eq!(err.code(), ErrorCode::AuthorizationError);
        assert_eq!(err.code().status_code(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_secure_error_rate_limit() {
        let err = SecureError::rate_limit("User exceeded 100 requests per minute");

        assert_eq!(err.code(), ErrorCode::RateLimitExceeded);
        assert!(err.public_details.is_some());
        // Public message should not contain specific rate limit details
        assert!(!err.public_details.as_ref().unwrap().contains("100"));
    }

    #[test]
    fn test_secure_error_bad_request() {
        let err = SecureError::bad_request("Invalid JSON format");

        assert_eq!(err.code(), ErrorCode::BadRequest);
        assert!(err.public_details.is_some());
        assert!(err
            .public_details
            .as_ref()
            .unwrap()
            .contains("Invalid JSON"));
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(format!("{}", ErrorCode::DatabaseError), "DatabaseError");
        assert_eq!(format!("{}", ErrorCode::BadRequest), "BadRequest");
    }
}
