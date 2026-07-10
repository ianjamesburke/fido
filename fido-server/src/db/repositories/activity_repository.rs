use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::db::DbPool;

/// Gates GitHub activity fetches. Synced items live in `posts`.
#[derive(Clone)]
pub struct ActivityRepository {
    pool: DbPool,
}

impl ActivityRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn get_last_synced_at(&self, community_id: Uuid) -> Result<Option<DateTime<Utc>>> {
        let conn = self.pool.get()?;
        let value: Option<String> = conn
            .query_row(
                "SELECT last_synced_at FROM community_activity_sync WHERE community_id = ?1",
                [community_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        value
            .map(|value| Ok(DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc)))
            .transpose()
    }

    pub fn mark_synced(&self, community_id: Uuid, at: DateTime<Utc>) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO community_activity_sync (community_id, last_synced_at)
             VALUES (?1, ?2)
             ON CONFLICT(community_id) DO UPDATE SET last_synced_at = excluded.last_synced_at",
            (community_id.to_string(), at.to_rfc3339()),
        )?;
        Ok(())
    }
}
