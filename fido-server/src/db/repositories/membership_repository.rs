#![allow(dead_code)] // v2 repository is covered by sqlite-tests before its API surface is wired.

use anyhow::{Context, Result};
use rusqlite::types::Type;
use rusqlite::OptionalExtension;
use uuid::Uuid;

use fido_types::{Membership, MembershipRole};

use crate::db::{row, DbPool};

#[derive(Clone)]
pub struct MembershipRepository {
    pool: DbPool,
}

impl MembershipRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert a membership
    pub fn insert(&self, membership: &Membership) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO memberships (community_id, user_id, role, created_at) VALUES (?, ?, ?, ?)",
            (
                membership.community_id.to_string(),
                membership.user_id.to_string(),
                membership.role.as_str(),
                membership.created_at.to_rfc3339(),
            ),
        )
        .context("Failed to insert membership")?;
        Ok(())
    }

    /// Insert a membership if it does not already exist
    pub fn insert_if_missing(&self, membership: &Membership) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO memberships (community_id, user_id, role, created_at) VALUES (?, ?, ?, ?)",
            (
                membership.community_id.to_string(),
                membership.user_id.to_string(),
                membership.role.as_str(),
                membership.created_at.to_rfc3339(),
            ),
        )
        .context("Failed to insert membership if missing")?;
        Ok(())
    }

    /// Get a single membership
    pub fn get(&self, community_id: &Uuid, user_id: &Uuid) -> Result<Option<Membership>> {
        let conn = self.pool.get()?;
        let membership = conn
            .query_row(
                "SELECT community_id, user_id, role, created_at FROM memberships
                 WHERE community_id = ? AND user_id = ?",
                (community_id.to_string(), user_id.to_string()),
                map_membership_row,
            )
            .optional()
            .context("Failed to fetch membership")?;
        Ok(membership)
    }

    /// Update a member's role
    pub fn update_role(
        &self,
        community_id: &Uuid,
        user_id: &Uuid,
        role: MembershipRole,
    ) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE memberships SET role = ? WHERE community_id = ? AND user_id = ?",
            (role.as_str(), community_id.to_string(), user_id.to_string()),
        )
        .context("Failed to update membership role")?;
        Ok(())
    }

    /// Delete a membership (leave a community)
    pub fn delete(&self, community_id: &Uuid, user_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM memberships WHERE community_id = ? AND user_id = ?",
            (community_id.to_string(), user_id.to_string()),
        )
        .context("Failed to delete membership")?;
        Ok(())
    }

    /// List all memberships for a user
    pub fn list_for_user(&self, user_id: &Uuid) -> Result<Vec<Membership>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT community_id, user_id, role, created_at FROM memberships
             WHERE user_id = ? ORDER BY created_at DESC",
        )?;
        let memberships = stmt
            .query_map([user_id.to_string()], map_membership_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(memberships)
    }

    /// List admin memberships for a community
    pub fn list_admins(&self, community_id: &Uuid) -> Result<Vec<Membership>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT community_id, user_id, role, created_at FROM memberships
             WHERE community_id = ? AND role = 'admin' ORDER BY created_at ASC",
        )?;
        let memberships = stmt
            .query_map([community_id.to_string()], map_membership_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(memberships)
    }
}

fn map_membership_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Membership> {
    let community_id_str: String = row.get(0)?;
    let user_id_str: String = row.get(1)?;
    let role_str: String = row.get(2)?;
    let created_at_str: String = row.get(3)?;

    let role = MembershipRole::parse(&role_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid membership role: {role_str}"),
            )),
        )
    })?;

    Ok(Membership {
        community_id: row::parse_uuid(&community_id_str, 0)?,
        user_id: row::parse_uuid(&user_id_str, 1)?,
        role,
        created_at: row::parse_datetime(&created_at_str, 3)?,
    })
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;
    use chrono::Utc;

    fn setup() -> Result<(Database, Uuid, Uuid)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let conn = db.pool.get()?;

        let user_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user) VALUES (?, ?, ?, ?)",
            (user_id.to_string(), "member1", "2024-01-01T00:00:00Z", 1),
        )?;

        let community_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO communities (id, github_repo_id, owner, name, require_thread_approval, created_at)
             VALUES (?, ?, ?, ?, 0, ?)",
            (community_id.to_string(), 1_i64, "octocat", "hello", "2024-01-01T00:00:00Z"),
        )?;

        Ok((db, community_id, user_id))
    }

    fn membership(community_id: Uuid, user_id: Uuid, role: MembershipRole) -> Membership {
        Membership {
            community_id,
            user_id,
            role,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_insert_get_update_delete() -> Result<()> {
        let (db, community_id, user_id) = setup()?;
        let repo = MembershipRepository::new(db.pool.clone());

        repo.insert(&membership(community_id, user_id, MembershipRole::Member))?;
        let got = repo
            .get(&community_id, &user_id)?
            .expect("membership exists");
        assert_eq!(got.role, MembershipRole::Member);

        repo.update_role(&community_id, &user_id, MembershipRole::Admin)?;
        let got = repo
            .get(&community_id, &user_id)?
            .expect("membership exists");
        assert_eq!(got.role, MembershipRole::Admin);

        assert_eq!(repo.list_for_user(&user_id)?.len(), 1);
        assert_eq!(repo.list_admins(&community_id)?.len(), 1);

        repo.delete(&community_id, &user_id)?;
        assert!(repo.get(&community_id, &user_id)?.is_none());
        Ok(())
    }
}
