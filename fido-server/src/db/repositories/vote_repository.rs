use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::OptionalExtension;
use uuid::Uuid;

use fido_types::{Vote, VoteDirection};

use crate::db::DbPool;

pub struct VoteRepository {
    pool: DbPool,
}

impl VoteRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Upsert a vote (insert or update if exists)
    pub fn upsert_vote(&self, user_id: &Uuid, post_id: &Uuid, direction: VoteDirection) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute(
            "INSERT INTO votes (user_id, post_id, direction, created_at) 
             VALUES (?, ?, ?, ?)
             ON CONFLICT(user_id, post_id) 
             DO UPDATE SET direction = excluded.direction, created_at = excluded.created_at",
            (
                user_id.to_string(),
                post_id.to_string(),
                direction.as_str(),
                Utc::now().to_rfc3339(),
            ),
        ).context("Failed to upsert vote")?;
        
        Ok(())
    }

    /// Get a user's vote on a post
    pub fn get_vote(&self, user_id: &Uuid, post_id: &Uuid) -> Result<Option<Vote>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT user_id, post_id, direction, created_at 
             FROM votes 
             WHERE user_id = ? AND post_id = ?"
        )?;

        let vote = stmt.query_row(
            (user_id.to_string(), post_id.to_string()),
            |row| {
                let direction_str: String = row.get(2)?;
                Ok(Vote {
                    user_id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    post_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    direction: VoteDirection::parse(&direction_str).unwrap(),
                    created_at: row.get::<_, String>(3)?.parse().unwrap(),
                })
            }
        ).optional()?;

        Ok(vote)
    }

    /// Delete a vote
    #[allow(dead_code)]
    pub fn delete_vote(&self, user_id: &Uuid, post_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM votes WHERE user_id = ? AND post_id = ?",
            (user_id.to_string(), post_id.to_string()),
        ).context("Failed to delete vote")?;
        Ok(())
    }

    /// Calculate karma for a user (sum of upvotes on their posts)
    pub fn calculate_karma(&self, user_id: &Uuid) -> Result<i32> {
        let conn = self.pool.get()?;
        let karma: i32 = conn.query_row(
            "SELECT COUNT(*) 
             FROM votes v
             JOIN posts p ON v.post_id = p.id
             WHERE p.author_id = ? AND v.direction = 'up'",
            [user_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(karma)
    }
}
