#![allow(dead_code)] // v2 repository is covered by sqlite-tests before its API surface is wired.

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use fido_types::Channel;

use crate::db::{row, DbPool};

#[derive(Clone)]
pub struct ChannelRepository {
    pool: DbPool,
}

impl ChannelRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new channel
    pub fn create(&self, channel: &Channel) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO channels (id, community_id, name, created_at) VALUES (?, ?, ?, ?)",
            (
                channel.id.to_string(),
                channel.community_id.to_string(),
                &channel.name,
                channel.created_at.to_rfc3339(),
            ),
        )
        .context("Failed to create channel")?;
        Ok(())
    }

    /// Get a channel by its id
    pub fn get_by_id(&self, channel_id: &Uuid) -> Result<Option<Channel>> {
        let conn = self.pool.get()?;
        let channel = conn
            .query_row(
                "SELECT id, community_id, name, created_at FROM channels WHERE id = ?",
                [channel_id.to_string()],
                map_channel_row,
            )
            .optional()
            .context("Failed to fetch channel by id")?;
        Ok(channel)
    }

    /// List channels for a community
    pub fn list_by_community(&self, community_id: &Uuid) -> Result<Vec<Channel>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, community_id, name, created_at FROM channels
             WHERE community_id = ? ORDER BY created_at ASC",
        )?;
        let channels = stmt
            .query_map([community_id.to_string()], map_channel_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(channels)
    }
}

fn map_channel_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Channel> {
    let id_str: String = row.get(0)?;
    let community_id_str: String = row.get(1)?;
    let created_at_str: String = row.get(3)?;

    Ok(Channel {
        id: row::parse_uuid(&id_str, 0)?,
        community_id: row::parse_uuid(&community_id_str, 1)?,
        name: row.get(2)?,
        created_at: row::parse_datetime(&created_at_str, 3)?,
    })
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;
    use chrono::Utc;

    fn setup() -> Result<(Database, Uuid)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let community_id = Uuid::new_v4();
        let conn = db.pool.get()?;
        conn.execute(
            "INSERT INTO communities (id, github_repo_id, owner, name, require_thread_approval, created_at)
             VALUES (?, ?, ?, ?, 0, ?)",
            (
                community_id.to_string(),
                1_i64,
                "octocat",
                "hello",
                "2024-01-01T00:00:00Z",
            ),
        )?;
        Ok((db, community_id))
    }

    fn sample_channel(community_id: Uuid, name: &str) -> Channel {
        Channel {
            id: Uuid::new_v4(),
            community_id,
            name: name.to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_create_get_list() -> Result<()> {
        let (db, community_id) = setup()?;
        let repo = ChannelRepository::new(db.pool.clone());

        let general = sample_channel(community_id, "general");
        let random = sample_channel(community_id, "random");
        repo.create(&general)?;
        repo.create(&random)?;

        let fetched = repo.get_by_id(&general.id)?.expect("channel exists");
        assert_eq!(fetched.name, "general");

        let channels = repo.list_by_community(&community_id)?;
        assert_eq!(channels.len(), 2);
        Ok(())
    }

    #[test]
    fn test_unique_name_per_community() -> Result<()> {
        let (db, community_id) = setup()?;
        let repo = ChannelRepository::new(db.pool.clone());
        repo.create(&sample_channel(community_id, "general"))?;
        let dup = repo.create(&sample_channel(community_id, "general"));
        assert!(dup.is_err(), "duplicate channel name should be rejected");
        Ok(())
    }
}
