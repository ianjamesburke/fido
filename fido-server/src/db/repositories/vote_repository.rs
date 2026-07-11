use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::OptionalExtension;
use uuid::Uuid;

use fido_types::{Vote, VoteDirection};

use crate::db::{row, DbPool};

#[derive(Clone)]
pub struct VoteRepository {
    pool: DbPool,
}

impl VoteRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Upsert a vote (insert or update if exists)
    pub fn upsert_vote(
        &self,
        user_id: &Uuid,
        post_id: &Uuid,
        direction: VoteDirection,
    ) -> Result<()> {
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
        )
        .context("Failed to upsert vote")?;

        Ok(())
    }

    /// Get a user's vote on a post
    pub fn get_vote(&self, user_id: &Uuid, post_id: &Uuid) -> Result<Option<Vote>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT user_id, post_id, direction, created_at 
             FROM votes 
             WHERE user_id = ? AND post_id = ?",
        )?;

        let vote = stmt
            .query_row((user_id.to_string(), post_id.to_string()), |row| {
                let user_id_str: String = row.get(0)?;
                let post_id_str: String = row.get(1)?;
                let direction_str: String = row.get(2)?;
                let created_at_str: String = row.get(3)?;

                Ok(Vote {
                    user_id: row::parse_uuid(&user_id_str, 0)?,
                    post_id: row::parse_uuid(&post_id_str, 1)?,
                    direction: row::parse_vote_direction(&direction_str, 2)?,
                    created_at: row::parse_datetime(&created_at_str, 3)?,
                })
            })
            .optional()?;

        Ok(vote)
    }

    /// Batch-fetch a user's votes for many posts in a single query.
    ///
    /// Replaces the per-post `get_vote` loop in feed/reply-tree rendering (an
    /// N+1). Placeholders are generated to match the id count; the ids are
    /// bound as parameters, never interpolated into the SQL.
    pub fn get_votes_for_posts(
        &self,
        user_id: &Uuid,
        post_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, VoteDirection>> {
        if post_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self.pool.get()?;
        let placeholders = vec!["?"; post_ids.len()].join(", ");
        let sql = format!(
            "SELECT post_id, direction FROM votes
             WHERE user_id = ? AND post_id IN ({placeholders})"
        );
        let mut stmt = conn.prepare(&sql)?;

        let mut params: Vec<String> = Vec::with_capacity(post_ids.len() + 1);
        params.push(user_id.to_string());
        params.extend(post_ids.iter().map(Uuid::to_string));
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let post_id_str: String = row.get(0)?;
            let direction_str: String = row.get(1)?;
            Ok((
                row::parse_uuid(&post_id_str, 0)?,
                row::parse_vote_direction(&direction_str, 1)?,
            ))
        })?;

        let mut votes = HashMap::with_capacity(post_ids.len());
        for row in rows {
            let (post_id, direction) = row?;
            votes.insert(post_id, direction);
        }
        Ok(votes)
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

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;
    use chrono::Utc;

    /// Seed one user, one community, and `n` posts by that user; return their ids.
    fn seed(db: &Database, n: usize) -> Result<(Uuid, Vec<Uuid>)> {
        let user_id = Uuid::new_v4();
        let community_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let conn = db.connection()?;
        conn.execute(
            "INSERT INTO users (id, username, bio, join_date, is_test_user, is_admin)
             VALUES (?, 'voter', NULL, ?, 1, 0)",
            (user_id.to_string(), &now),
        )?;
        conn.execute(
            "INSERT INTO communities (id, github_repo_id, owner, name, require_thread_approval, created_at)
             VALUES (?, 1, 'octocat', 'hello', 0, ?)",
            (community_id.to_string(), &now),
        )?;
        let mut post_ids = Vec::with_capacity(n);
        for i in 0..n {
            let post_id = Uuid::new_v4();
            conn.execute(
                "INSERT INTO posts (id, author_id, community_id, content, created_at)
                 VALUES (?, ?, ?, ?, ?)",
                (
                    post_id.to_string(),
                    user_id.to_string(),
                    community_id.to_string(),
                    format!("post {i}"),
                    &now,
                ),
            )?;
            post_ids.push(post_id);
        }
        Ok((user_id, post_ids))
    }

    #[test]
    fn get_votes_for_posts_batches_lookup_across_many_posts() -> Result<()> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let (user_id, posts) = seed(&db, 5)?;
        let repo = VoteRepository::new(db.pool.clone());

        // Vote on a subset; leave the rest unvoted.
        repo.upsert_vote(&user_id, &posts[0], VoteDirection::Up)?;
        repo.upsert_vote(&user_id, &posts[2], VoteDirection::Down)?;
        repo.upsert_vote(&user_id, &posts[4], VoteDirection::Up)?;

        // A single batched call resolves every post's vote (this replaces the
        // former per-post N+1 loop in feed/reply rendering).
        let votes = repo.get_votes_for_posts(&user_id, &posts)?;

        assert_eq!(votes.len(), 3);
        assert_eq!(votes.get(&posts[0]), Some(&VoteDirection::Up));
        assert_eq!(votes.get(&posts[2]), Some(&VoteDirection::Down));
        assert_eq!(votes.get(&posts[4]), Some(&VoteDirection::Up));
        assert!(!votes.contains_key(&posts[1]));
        assert!(!votes.contains_key(&posts[3]));
        Ok(())
    }

    #[test]
    fn get_votes_for_posts_empty_input_is_noop() -> Result<()> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let repo = VoteRepository::new(db.pool.clone());
        let votes = repo.get_votes_for_posts(&Uuid::new_v4(), &[])?;
        assert!(votes.is_empty());
        Ok(())
    }
}
