use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::db::DbPool;

/// A persisted session row.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub last_activity: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct SessionRepository {
    pool: DbPool,
}

impl SessionRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// `token_hash` is the SHA-256 hex digest of the raw session token. The
    /// `token` column stores this hash; the raw token is never persisted.
    pub fn create_session(
        &self,
        token_hash: &str,
        user_id: Uuid,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        last_activity: DateTime<Utc>,
    ) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO sessions (token, user_id, created_at, expires_at, last_activity) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                token_hash,
                user_id.to_string(),
                created_at.to_rfc3339(),
                expires_at.to_rfc3339(),
                last_activity.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_session(&self, token_hash: &str) -> Result<Option<SessionRecord>> {
        let conn = self.pool.get()?;
        let row: Option<(String, String, Option<String>)> = conn
            .query_row(
                "SELECT user_id, expires_at, last_activity FROM sessions WHERE token = ?1",
                rusqlite::params![token_hash],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;

        match row {
            Some((user_id_str, expires_at_str, last_activity_str)) => {
                let user_id = Uuid::parse_str(&user_id_str)?;
                let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&Utc);
                let last_activity = match last_activity_str {
                    Some(v) => Some(DateTime::parse_from_rfc3339(&v)?.with_timezone(&Utc)),
                    None => None,
                };
                Ok(Some(SessionRecord {
                    user_id,
                    expires_at,
                    last_activity,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn update_activity(&self, token_hash: &str, at: DateTime<Utc>) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE sessions SET last_activity = ?1 WHERE token = ?2",
            rusqlite::params![at.to_rfc3339(), token_hash],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, token_hash: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        Ok(conn.execute(
            "DELETE FROM sessions WHERE token = ?1",
            rusqlite::params![token_hash],
        )?)
    }

    pub fn cleanup_expired_sessions(&self, now: DateTime<Utc>) -> Result<usize> {
        let conn = self.pool.get()?;
        Ok(conn.execute(
            "DELETE FROM sessions WHERE expires_at < ?1",
            rusqlite::params![now.to_rfc3339()],
        )?)
    }

    #[allow(dead_code)]
    pub fn invalidate_user_sessions(&self, user_id: Uuid) -> Result<usize> {
        let conn = self.pool.get()?;
        Ok(conn.execute(
            "DELETE FROM sessions WHERE user_id = ?1",
            rusqlite::params![user_id.to_string()],
        )?)
    }
}
