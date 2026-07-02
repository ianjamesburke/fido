use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::DbPool;

#[derive(Clone)]
pub struct AuditRepository {
    pool: DbPool,
}

impl AuditRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Record a security audit event and return its generated id.
    pub fn log_event(
        &self,
        event_type: &str,
        user_id: Option<Uuid>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        details: Option<&str>,
        timestamp: DateTime<Utc>,
    ) -> Result<Uuid> {
        let conn = self.pool.get()?;
        let id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO audit_logs (id, event_type, user_id, ip_address, user_agent, details, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id.to_string(),
                event_type,
                user_id.map(|u| u.to_string()),
                ip_address,
                user_agent,
                details,
                timestamp.to_rfc3339(),
            ],
        )?;
        Ok(id)
    }
}
