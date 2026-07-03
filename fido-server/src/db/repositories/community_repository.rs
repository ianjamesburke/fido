#![allow(dead_code)] // v2 repository is covered by sqlite-tests before its API surface is wired.

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use fido_types::Community;

use crate::db::{row, DbPool};

#[derive(Clone)]
pub struct CommunityRepository {
    pool: DbPool,
}

impl CommunityRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new community
    pub fn create(&self, community: &Community) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO communities (id, github_repo_id, owner, name, claimed_by, require_thread_approval, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                community.id.to_string(),
                community.github_repo_id,
                &community.owner,
                &community.name,
                community.claimed_by.map(|id| id.to_string()),
                if community.require_thread_approval { 1 } else { 0 },
                community.created_at.to_rfc3339(),
            ),
        )
        .context("Failed to create community")?;
        Ok(())
    }

    /// Get a community by its id
    pub fn get_by_id(&self, community_id: &Uuid) -> Result<Option<Community>> {
        let conn = self.pool.get()?;
        let community = conn
            .query_row(
                "SELECT id, github_repo_id, owner, name, claimed_by, require_thread_approval, created_at
                 FROM communities WHERE id = ?",
                [community_id.to_string()],
                map_community_row,
            )
            .optional()
            .context("Failed to fetch community by id")?;
        Ok(community)
    }

    /// Get a community by its owner/name pair
    pub fn get_by_owner_name(&self, owner: &str, name: &str) -> Result<Option<Community>> {
        let conn = self.pool.get()?;
        let community = conn
            .query_row(
                "SELECT id, github_repo_id, owner, name, claimed_by, require_thread_approval, created_at
                 FROM communities WHERE owner = ? AND name = ?",
                [owner, name],
                map_community_row,
            )
            .optional()
            .context("Failed to fetch community by owner/name")?;
        Ok(community)
    }

    /// Get a community by its GitHub numeric repo id
    pub fn get_by_github_repo_id(&self, github_repo_id: i64) -> Result<Option<Community>> {
        let conn = self.pool.get()?;
        let community = conn
            .query_row(
                "SELECT id, github_repo_id, owner, name, claimed_by, require_thread_approval, created_at
                 FROM communities WHERE github_repo_id = ?",
                [github_repo_id],
                map_community_row,
            )
            .optional()
            .context("Failed to fetch community by github_repo_id")?;
        Ok(community)
    }

    /// List communities the user has joined
    pub fn list_joined(&self, user_id: &Uuid) -> Result<Vec<Community>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT c.id, c.github_repo_id, c.owner, c.name, c.claimed_by, c.require_thread_approval, c.created_at
             FROM communities c
             JOIN memberships m ON m.community_id = c.id
             WHERE m.user_id = ?
             ORDER BY c.created_at DESC",
        )?;
        let communities = stmt
            .query_map([user_id.to_string()], map_community_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(communities)
    }

    /// List all communities in deterministic display order
    pub fn list_all(&self) -> Result<Vec<Community>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, github_repo_id, owner, name, claimed_by, require_thread_approval, created_at
             FROM communities
             ORDER BY owner ASC, name ASC",
        )?;
        let communities = stmt
            .query_map([], map_community_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(communities)
    }

    /// Record the user who claimed the community
    pub fn set_claimed_by(&self, community_id: &Uuid, user_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE communities SET claimed_by = ? WHERE id = ?",
            (user_id.to_string(), community_id.to_string()),
        )
        .context("Failed to set community claimed_by")?;
        Ok(())
    }

    /// Count members of a community
    pub fn member_count(&self, community_id: &Uuid) -> Result<i64> {
        let conn = self.pool.get()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memberships WHERE community_id = ?",
            [community_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

fn map_community_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Community> {
    let id_str: String = row.get(0)?;
    let claimed_by_str: Option<String> = row.get(4)?;
    let created_at_str: String = row.get(6)?;

    Ok(Community {
        id: row::parse_uuid(&id_str, 0)?,
        github_repo_id: row.get(1)?,
        owner: row.get(2)?,
        name: row.get(3)?,
        claimed_by: row::parse_optional_uuid(claimed_by_str.as_deref(), 4)?,
        require_thread_approval: row.get::<_, i32>(5)? == 1,
        created_at: row::parse_datetime(&created_at_str, 6)?,
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
        let user_id = Uuid::new_v4();
        let conn = db.pool.get()?;
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user) VALUES (?, ?, ?, ?)",
            (user_id.to_string(), "owner1", "2024-01-01T00:00:00Z", 1),
        )?;
        Ok((db, user_id))
    }

    fn sample_community(github_repo_id: i64) -> Community {
        Community {
            id: Uuid::new_v4(),
            github_repo_id,
            owner: "octocat".to_string(),
            name: "hello".to_string(),
            claimed_by: None,
            require_thread_approval: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_create_and_get() -> Result<()> {
        let (db, _user) = setup()?;
        let repo = CommunityRepository::new(db.pool.clone());
        let community = sample_community(42);
        repo.create(&community)?;

        let fetched = repo.get_by_id(&community.id)?.expect("community exists");
        assert_eq!(fetched.github_repo_id, 42);
        assert_eq!(fetched.owner, "octocat");

        let by_repo = repo
            .get_by_github_repo_id(42)?
            .expect("community exists by repo id");
        assert_eq!(by_repo.id, community.id);

        let by_owner_name = repo
            .get_by_owner_name("octocat", "hello")?
            .expect("community exists by owner/name");
        assert_eq!(by_owner_name.id, community.id);
        assert!(repo.get_by_owner_name("octocat", "missing")?.is_none());
        Ok(())
    }

    #[test]
    fn test_set_claimed_by_and_list_joined() -> Result<()> {
        let (db, user_id) = setup()?;
        let repo = CommunityRepository::new(db.pool.clone());
        let community = sample_community(7);
        repo.create(&community)?;

        repo.set_claimed_by(&community.id, &user_id)?;
        let fetched = repo.get_by_id(&community.id)?.expect("community exists");
        assert_eq!(fetched.claimed_by, Some(user_id));

        // No membership yet
        assert!(repo.list_joined(&user_id)?.is_empty());

        // Add a membership then list
        let conn = db.pool.get()?;
        conn.execute(
            "INSERT INTO memberships (community_id, user_id, role, created_at) VALUES (?, ?, 'admin', ?)",
            (community.id.to_string(), user_id.to_string(), "2024-01-01T00:00:00Z"),
        )?;
        let joined = repo.list_joined(&user_id)?;
        assert_eq!(joined.len(), 1);
        assert_eq!(repo.member_count(&community.id)?, 1);
        Ok(())
    }
}
