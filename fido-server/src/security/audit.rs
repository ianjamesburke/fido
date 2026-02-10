//! Audit logging module for security event tracking
//!
//! This module provides comprehensive audit logging for security-relevant events
//! such as authentication, session management, and suspicious activity detection.

use crate::stores::AuditStore;
use chrono::Utc;
use std::fmt;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during audit logging
#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Store error: {0}")]
    Store(String),
}

/// Result type for audit operations
pub type AuditResult<T> = Result<T, AuditError>;

/// Types of security events that can be audited
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventType {
    /// User successfully logged in
    LoginSuccess,
    /// User failed to authenticate
    LoginFailure,
    /// Session was revoked/invalidated
    SessionRevoked,
    /// Device code was generated for OAuth flow
    DeviceCodeGenerated,
    /// Device code was successfully used for authentication
    DeviceCodeUsed,
    /// Rate limit was exceeded
    RateLimitExceeded,
    /// Validation failure occurred
    ValidationFailure,
    /// Admin action was performed
    AdminAction,
}

impl fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditEventType::LoginSuccess => write!(f, "login_success"),
            AuditEventType::LoginFailure => write!(f, "login_failure"),
            AuditEventType::SessionRevoked => write!(f, "session_revoked"),
            AuditEventType::DeviceCodeGenerated => write!(f, "device_code_generated"),
            AuditEventType::DeviceCodeUsed => write!(f, "device_code_used"),
            AuditEventType::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            AuditEventType::ValidationFailure => write!(f, "validation_failure"),
            AuditEventType::AdminAction => write!(f, "admin_action"),
        }
    }
}

/// An audit event to be logged
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Type of security event
    pub event_type: AuditEventType,
    /// User ID associated with the event (if applicable)
    pub user_id: Option<Uuid>,
    /// IP address of the client
    pub ip_address: Option<String>,
    /// User-Agent header from the client
    pub user_agent: Option<String>,
    /// Additional details about the event (JSON or plain text)
    pub details: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            event_type,
            user_id: None,
            ip_address: None,
            user_agent: None,
            details: None,
        }
    }

    /// Set the user ID for this event
    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set the user ID from an optional value
    pub fn with_optional_user_id(mut self, user_id: Option<Uuid>) -> Self {
        self.user_id = user_id;
        self
    }

    /// Set the IP address from an optional value
    pub fn with_optional_ip_address(mut self, ip: Option<String>) -> Self {
        self.ip_address = ip;
        self
    }

    /// Set the User-Agent from an optional value
    pub fn with_optional_user_agent(mut self, ua: Option<String>) -> Self {
        self.user_agent = ua;
        self
    }

    /// Set additional details for this event
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Audit logger for recording security events
#[derive(Clone)]
pub struct AuditLogger {
    store: Arc<dyn AuditStore>,
}

impl AuditLogger {
    /// Create a new AuditLogger with the given audit store
    pub fn new(store: Arc<dyn AuditStore>) -> Self {
        Self { store }
    }

    /// Log a security event to the audit log
    pub fn log(&self, event: AuditEvent) -> AuditResult<Uuid> {
        self.store
            .log_event(
                &event.event_type.to_string(),
                event.user_id,
                event.ip_address.as_deref(),
                event.user_agent.as_deref(),
                event.details.as_deref(),
                Utc::now(),
            )
            .map_err(|e| AuditError::Store(e.to_string()))
    }
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::stores::sqlite::SqliteAuditStore;

    fn create_test_logger(db: &Database) -> AuditLogger {
        AuditLogger::new(Arc::new(SqliteAuditStore::new(db.pool.clone())))
    }

    #[test]
    fn test_audit_event_type_display() {
        assert_eq!(AuditEventType::LoginSuccess.to_string(), "login_success");
        assert_eq!(AuditEventType::LoginFailure.to_string(), "login_failure");
        assert_eq!(
            AuditEventType::SessionRevoked.to_string(),
            "session_revoked"
        );
        assert_eq!(
            AuditEventType::DeviceCodeGenerated.to_string(),
            "device_code_generated"
        );
        assert_eq!(
            AuditEventType::DeviceCodeUsed.to_string(),
            "device_code_used"
        );
    }

    #[test]
    fn test_audit_event_builder() {
        let user_id = Uuid::new_v4();
        let event = AuditEvent::new(AuditEventType::LoginSuccess)
            .with_user_id(user_id)
            .with_optional_ip_address(Some("192.168.1.1".to_string()))
            .with_optional_user_agent(Some("Mozilla/5.0".to_string()))
            .with_details("Test login");

        assert_eq!(event.event_type, AuditEventType::LoginSuccess);
        assert_eq!(event.user_id, Some(user_id));
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(event.user_agent, Some("Mozilla/5.0".to_string()));
        assert_eq!(event.details, Some("Test login".to_string()));
    }

    #[test]
    fn test_audit_logger_log() {
        let db = Database::in_memory().expect("Failed to create test database");
        db.initialize().expect("Failed to initialize database");
        let logger = create_test_logger(&db);

        let user_id = Uuid::new_v4();
        let event = AuditEvent::new(AuditEventType::LoginSuccess)
            .with_user_id(user_id)
            .with_optional_ip_address(Some("192.168.1.1".to_string()))
            .with_details("Successful login");

        let result = logger.log(event);
        assert!(result.is_ok());

        // Verify the log was stored
        let conn = db.connection().expect("Failed to get connection");
        let (count, event_type, logged_user_id, ip_address): (
            i64,
            String,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT COUNT(*), event_type, user_id, ip_address FROM audit_logs",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("Failed to query audit log");

        assert_eq!(count, 1);
        assert_eq!(event_type, "login_success");
        assert_eq!(logged_user_id, Some(user_id.to_string()));
        assert_eq!(ip_address, Some("192.168.1.1".to_string()));
    }
}
