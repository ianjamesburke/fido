use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::db::DbPool;

#[derive(Clone)]
pub struct RateLimitRepository {
    pool: DbPool,
}

impl RateLimitRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get the timestamp of the user's most recent post, if any.
    pub fn get_last_post_at(&self, user_id: &Uuid) -> Result<Option<DateTime<Utc>>> {
        let conn = self.pool.get()?;
        let last_post_at: Option<String> = conn
            .query_row(
                "SELECT last_post_at FROM post_rate_limits WHERE user_id = ?",
                [user_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        match last_post_at {
            Some(value) => {
                let parsed = DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc);
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Record the timestamp of the user's most recent post.
    pub fn update_last_post_at(&self, user_id: &Uuid, at: DateTime<Utc>) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO post_rate_limits (user_id, last_post_at) VALUES (?, ?)
             ON CONFLICT(user_id) DO UPDATE SET last_post_at = excluded.last_post_at",
            (user_id.to_string(), at.to_rfc3339()),
        )?;
        Ok(())
    }

    /// Get the timestamp of the user's most recent direct message, if any.
    pub fn get_last_dm_at(&self, user_id: &Uuid) -> Result<Option<DateTime<Utc>>> {
        let conn = self.pool.get()?;
        let last_dm_at: Option<String> = conn
            .query_row(
                "SELECT last_dm_at FROM dm_rate_limits WHERE user_id = ?",
                [user_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        match last_dm_at {
            Some(value) => {
                let parsed = DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc);
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Record the timestamp of the user's most recent direct message.
    pub fn update_last_dm_at(&self, user_id: &Uuid, at: DateTime<Utc>) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO dm_rate_limits (user_id, last_dm_at) VALUES (?, ?)
             ON CONFLICT(user_id) DO UPDATE SET last_dm_at = excluded.last_dm_at",
            (user_id.to_string(), at.to_rfc3339()),
        )?;
        Ok(())
    }
}
