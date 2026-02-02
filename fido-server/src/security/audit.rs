//! Audit logging module for security event tracking
//!
//! This module provides comprehensive audit logging for security-relevant events
//! such as authentication, session management, and suspicious activity detection.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during audit logging
#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Failed to serialize audit details: {0}")]
    Serialization(String),
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
    /// New session was created
    SessionCreated,
    /// Session was revoked/invalidated
    SessionRevoked,
    /// Session was refreshed with new tokens
    SessionRefreshed,
    /// Suspicious activity was detected (e.g., IP change, unusual patterns)
    SuspiciousActivity,
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
            AuditEventType::SessionCreated => write!(f, "session_created"),
            AuditEventType::SessionRevoked => write!(f, "session_revoked"),
            AuditEventType::SessionRefreshed => write!(f, "session_refreshed"),
            AuditEventType::SuspiciousActivity => write!(f, "suspicious_activity"),
            AuditEventType::DeviceCodeGenerated => write!(f, "device_code_generated"),
            AuditEventType::DeviceCodeUsed => write!(f, "device_code_used"),
            AuditEventType::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            AuditEventType::ValidationFailure => write!(f, "validation_failure"),
            AuditEventType::AdminAction => write!(f, "admin_action"),
        }
    }
}

impl AuditEventType {
    /// Parse an event type from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "login_success" => Some(AuditEventType::LoginSuccess),
            "login_failure" => Some(AuditEventType::LoginFailure),
            "session_created" => Some(AuditEventType::SessionCreated),
            "session_revoked" => Some(AuditEventType::SessionRevoked),
            "session_refreshed" => Some(AuditEventType::SessionRefreshed),
            "suspicious_activity" => Some(AuditEventType::SuspiciousActivity),
            "device_code_generated" => Some(AuditEventType::DeviceCodeGenerated),
            "device_code_used" => Some(AuditEventType::DeviceCodeUsed),
            "rate_limit_exceeded" => Some(AuditEventType::RateLimitExceeded),
            "validation_failure" => Some(AuditEventType::ValidationFailure),
            "admin_action" => Some(AuditEventType::AdminAction),
            _ => None,
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

    /// Set the IP address for this event
    pub fn with_ip_address(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    /// Set the IP address from an optional value
    pub fn with_optional_ip_address(mut self, ip: Option<String>) -> Self {
        self.ip_address = ip;
        self
    }

    /// Set the User-Agent for this event
    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
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

    /// Set details from an optional value
    pub fn with_optional_details(mut self, details: Option<String>) -> Self {
        self.details = details;
        self
    }
}

/// A stored audit log entry retrieved from the database
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    /// Unique identifier for the log entry
    pub id: Uuid,
    /// Type of security event
    pub event_type: String,
    /// User ID associated with the event (if applicable)
    pub user_id: Option<Uuid>,
    /// IP address of the client
    pub ip_address: Option<String>,
    /// User-Agent header from the client
    pub user_agent: Option<String>,
    /// Additional details about the event
    pub details: Option<String>,
    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,
}

/// Audit logger for recording security events
#[derive(Clone)]
pub struct AuditLogger {
    pool: Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
}

impl AuditLogger {
    /// Create a new AuditLogger with the given database connection pool
    pub fn new(pool: Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>) -> Self {
        Self { pool }
    }

    /// Log a security event to the audit log
    pub fn log(&self, event: AuditEvent) -> AuditResult<Uuid> {
        let conn = self.pool.get().map_err(|e| {
            AuditError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
                Some(format!("Failed to get connection: {}", e)),
            ))
        })?;

        self.log_with_conn(&conn, event)
    }

    /// Log a security event using an existing connection
    pub fn log_with_conn(&self, conn: &Connection, event: AuditEvent) -> AuditResult<Uuid> {
        let id = Uuid::new_v4();
        let timestamp = Utc::now().to_rfc3339();
        let event_type = event.event_type.to_string();
        let user_id = event.user_id.map(|u| u.to_string());

        conn.execute(
            "INSERT INTO audit_logs (id, event_type, user_id, ip_address, user_agent, details, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id.to_string(),
                event_type,
                user_id,
                event.ip_address,
                event.user_agent,
                event.details,
                timestamp,
            ],
        )?;

        // Also log to tracing for immediate visibility
        tracing::info!(
            event_type = %event.event_type,
            user_id = ?event.user_id,
            ip_address = ?event.ip_address,
            details = ?event.details,
            "Audit event logged"
        );

        Ok(id)
    }

    /// Get recent audit log entries
    pub fn get_recent(&self, limit: usize) -> AuditResult<Vec<AuditLogEntry>> {
        let conn = self.pool.get().map_err(|e| {
            AuditError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
                Some(format!("Failed to get connection: {}", e)),
            ))
        })?;

        self.get_recent_with_conn(&conn, limit)
    }

    /// Get recent audit log entries using an existing connection
    pub fn get_recent_with_conn(
        &self,
        conn: &Connection,
        limit: usize,
    ) -> AuditResult<Vec<AuditLogEntry>> {
        let mut stmt = conn.prepare(
            "SELECT id, event_type, user_id, ip_address, user_agent, details, timestamp
             FROM audit_logs
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        let entries = stmt
            .query_map(params![limit as i64], |row| {
                let id_str: String = row.get(0)?;
                let user_id_str: Option<String> = row.get(2)?;
                let timestamp_str: String = row.get(6)?;

                Ok(AuditLogEntry {
                    id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::nil()),
                    event_type: row.get(1)?,
                    user_id: user_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    ip_address: row.get(3)?,
                    user_agent: row.get(4)?,
                    details: row.get(5)?,
                    timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Get audit log entries for a specific user
    pub fn get_by_user(&self, user_id: Uuid, limit: usize) -> AuditResult<Vec<AuditLogEntry>> {
        let conn = self.pool.get().map_err(|e| {
            AuditError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
                Some(format!("Failed to get connection: {}", e)),
            ))
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, event_type, user_id, ip_address, user_agent, details, timestamp
             FROM audit_logs
             WHERE user_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let entries = stmt
            .query_map(params![user_id.to_string(), limit as i64], |row| {
                let id_str: String = row.get(0)?;
                let user_id_str: Option<String> = row.get(2)?;
                let timestamp_str: String = row.get(6)?;

                Ok(AuditLogEntry {
                    id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::nil()),
                    event_type: row.get(1)?,
                    user_id: user_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    ip_address: row.get(3)?,
                    user_agent: row.get(4)?,
                    details: row.get(5)?,
                    timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Get audit log entries by event type
    pub fn get_by_event_type(
        &self,
        event_type: AuditEventType,
        limit: usize,
    ) -> AuditResult<Vec<AuditLogEntry>> {
        let conn = self.pool.get().map_err(|e| {
            AuditError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
                Some(format!("Failed to get connection: {}", e)),
            ))
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, event_type, user_id, ip_address, user_agent, details, timestamp
             FROM audit_logs
             WHERE event_type = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let entries = stmt
            .query_map(params![event_type.to_string(), limit as i64], |row| {
                let id_str: String = row.get(0)?;
                let user_id_str: Option<String> = row.get(2)?;
                let timestamp_str: String = row.get(6)?;

                Ok(AuditLogEntry {
                    id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::nil()),
                    event_type: row.get(1)?,
                    user_id: user_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    ip_address: row.get(3)?,
                    user_agent: row.get(4)?,
                    details: row.get(5)?,
                    timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r2d2_sqlite::SqliteConnectionManager;

    fn create_test_pool() -> Arc<r2d2::Pool<SqliteConnectionManager>> {
        let manager = SqliteConnectionManager::memory();
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("Failed to create pool");

        // Create the audit_logs table
        let conn = pool.get().expect("Failed to get connection");
        conn.execute(
            "CREATE TABLE audit_logs (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                user_id TEXT,
                ip_address TEXT,
                user_agent TEXT,
                details TEXT,
                timestamp TEXT NOT NULL
            )",
            [],
        )
        .expect("Failed to create table");

        Arc::new(pool)
    }

    #[test]
    fn test_audit_event_type_display() {
        assert_eq!(AuditEventType::LoginSuccess.to_string(), "login_success");
        assert_eq!(AuditEventType::LoginFailure.to_string(), "login_failure");
        assert_eq!(AuditEventType::SessionCreated.to_string(), "session_created");
        assert_eq!(AuditEventType::SessionRevoked.to_string(), "session_revoked");
        assert_eq!(
            AuditEventType::SessionRefreshed.to_string(),
            "session_refreshed"
        );
        assert_eq!(
            AuditEventType::SuspiciousActivity.to_string(),
            "suspicious_activity"
        );
        assert_eq!(
            AuditEventType::DeviceCodeGenerated.to_string(),
            "device_code_generated"
        );
        assert_eq!(AuditEventType::DeviceCodeUsed.to_string(), "device_code_used");
    }

    #[test]
    fn test_audit_event_type_from_str() {
        assert_eq!(
            AuditEventType::from_str("login_success"),
            Some(AuditEventType::LoginSuccess)
        );
        assert_eq!(
            AuditEventType::from_str("login_failure"),
            Some(AuditEventType::LoginFailure)
        );
        assert_eq!(AuditEventType::from_str("invalid"), None);
    }

    #[test]
    fn test_audit_event_builder() {
        let user_id = Uuid::new_v4();
        let event = AuditEvent::new(AuditEventType::LoginSuccess)
            .with_user_id(user_id)
            .with_ip_address("192.168.1.1")
            .with_user_agent("Mozilla/5.0")
            .with_details("Test login");

        assert_eq!(event.event_type, AuditEventType::LoginSuccess);
        assert_eq!(event.user_id, Some(user_id));
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(event.user_agent, Some("Mozilla/5.0".to_string()));
        assert_eq!(event.details, Some("Test login".to_string()));
    }

    #[test]
    fn test_audit_logger_log() {
        let pool = create_test_pool();
        let logger = AuditLogger::new(pool);

        let user_id = Uuid::new_v4();
        let event = AuditEvent::new(AuditEventType::LoginSuccess)
            .with_user_id(user_id)
            .with_ip_address("192.168.1.1")
            .with_details("Successful login");

        let result = logger.log(event);
        assert!(result.is_ok());

        // Verify the log was stored
        let entries = logger.get_recent(10).expect("Failed to get entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event_type, "login_success");
        assert_eq!(entries[0].user_id, Some(user_id));
        assert_eq!(entries[0].ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_audit_logger_get_by_user() {
        let pool = create_test_pool();
        let logger = AuditLogger::new(pool);

        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();

        // Log events for different users
        logger
            .log(AuditEvent::new(AuditEventType::LoginSuccess).with_user_id(user_id))
            .expect("Failed to log");
        logger
            .log(AuditEvent::new(AuditEventType::SessionCreated).with_user_id(user_id))
            .expect("Failed to log");
        logger
            .log(AuditEvent::new(AuditEventType::LoginSuccess).with_user_id(other_user_id))
            .expect("Failed to log");

        // Get entries for specific user
        let entries = logger
            .get_by_user(user_id, 10)
            .expect("Failed to get entries");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.user_id == Some(user_id)));
    }

    #[test]
    fn test_audit_logger_get_by_event_type() {
        let pool = create_test_pool();
        let logger = AuditLogger::new(pool);

        // Log different event types
        logger
            .log(AuditEvent::new(AuditEventType::LoginSuccess))
            .expect("Failed to log");
        logger
            .log(AuditEvent::new(AuditEventType::LoginSuccess))
            .expect("Failed to log");
        logger
            .log(AuditEvent::new(AuditEventType::LoginFailure))
            .expect("Failed to log");

        // Get entries by event type
        let entries = logger
            .get_by_event_type(AuditEventType::LoginSuccess, 10)
            .expect("Failed to get entries");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.event_type == "login_success"));
    }

    #[test]
    fn test_audit_event_optional_fields() {
        let event = AuditEvent::new(AuditEventType::SuspiciousActivity)
            .with_optional_user_id(None)
            .with_optional_ip_address(Some("10.0.0.1".to_string()))
            .with_optional_user_agent(None)
            .with_optional_details(Some("Unusual pattern detected".to_string()));

        assert_eq!(event.user_id, None);
        assert_eq!(event.ip_address, Some("10.0.0.1".to_string()));
        assert_eq!(event.user_agent, None);
        assert_eq!(
            event.details,
            Some("Unusual pattern detected".to_string())
        );
    }
}
