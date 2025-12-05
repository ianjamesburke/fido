use anyhow::{Context, Result};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::db::DbPool;

pub struct FriendRepository {
    pool: DbPool,
}

impl FriendRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Check if user A is following user B
    pub fn is_following(&self, follower_id: &Uuid, following_id: &Uuid) -> Result<bool> {
        let conn = self.pool.get()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM follows WHERE follower_id = ? AND following_id = ?",
            (follower_id.to_string(), following_id.to_string()),
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Check if users are mutual friends (both follow each other)
    pub fn are_mutual_friends(&self, user_a: &Uuid, user_b: &Uuid) -> Result<bool> {
        Ok(self.is_following(user_a, user_b)? && self.is_following(user_b, user_a)?)
    }

    /// Follow a user
    pub fn follow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        conn.execute(
            "INSERT OR IGNORE INTO follows (follower_id, following_id, created_at) VALUES (?, ?, ?)",
            (follower_id.to_string(), following_id.to_string(), now),
        ).context("Failed to follow user")?;

        Ok(())
    }

    /// Unfollow a user
    pub fn unfollow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> Result<usize> {
        let conn = self.pool.get()?;
        let rows_affected = conn
            .execute(
                "DELETE FROM follows WHERE follower_id = ? AND following_id = ?",
                (follower_id.to_string(), following_id.to_string()),
            )
            .context("Failed to unfollow user")?;
        Ok(rows_affected)
    }

    /// Get list of users that this user is following
    pub fn get_following(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT following_id FROM follows WHERE follower_id = ? ORDER BY created_at DESC",
        )?;

        let following = stmt
            .query_map([user_id.to_string()], |row| {
                let following_id: String = row.get(0)?;
                Ok(Uuid::parse_str(&following_id).unwrap())
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(following)
    }

    /// Get list of users that follow this user
    pub fn get_followers(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT follower_id FROM follows WHERE following_id = ? ORDER BY created_at DESC",
        )?;

        let followers = stmt
            .query_map([user_id.to_string()], |row| {
                let follower_id: String = row.get(0)?;
                Ok(Uuid::parse_str(&follower_id).unwrap())
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(followers)
    }

    /// Get mutual friends (users who follow each other)
    pub fn get_mutual_friends(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT f1.following_id 
             FROM follows f1
             INNER JOIN follows f2 
                ON f1.follower_id = f2.following_id 
                AND f1.following_id = f2.follower_id
             WHERE f1.follower_id = ?
             ORDER BY f1.created_at DESC",
        )?;

        let friends = stmt
            .query_map([user_id.to_string()], |row| {
                let friend_id: String = row.get(0)?;
                Ok(Uuid::parse_str(&friend_id).unwrap())
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(friends)
    }

    /// Get follower count
    pub fn get_follower_count(&self, user_id: &Uuid) -> Result<usize> {
        let conn = self.pool.get()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM follows WHERE following_id = ?",
            [user_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get following count
    pub fn get_following_count(&self, user_id: &Uuid) -> Result<usize> {
        let conn = self.pool.get()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM follows WHERE follower_id = ?",
            [user_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    // ===== Legacy friendships table methods (for backward compatibility) =====

    /// Get user's friends list with timestamps (legacy)
    #[allow(dead_code)]
    pub fn get_friends_with_timestamps(&self, user_id: &Uuid) -> Result<Vec<(Uuid, i64)>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT friend_id, created_at FROM friendships WHERE user_id = ? ORDER BY created_at DESC"
        )?;

        let friends = stmt
            .query_map([user_id.to_string()], |row| {
                let friend_id: String = row.get(0)?;
                let created_at: i64 = row.get(1)?;
                Ok((Uuid::parse_str(&friend_id).unwrap(), created_at))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(friends)
    }
    /// Check if users are friends (legacy)
    #[allow(dead_code)]
    pub fn are_friends(&self, user_id: &Uuid, friend_id: &Uuid) -> Result<bool> {
        let conn = self.pool.get()?;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM friendships WHERE user_id = ? AND friend_id = ?",
            (user_id.to_string(), friend_id.to_string()),
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_test_db() -> (Database, FriendRepository) {
        let db = Database::in_memory().expect("Failed to create test database");
        db.seed_test_data().expect("Failed to seed test data");
        let pool = db.pool.clone();
        let repo = FriendRepository::new(pool);
        (db, repo)
    }

    // TODO: Fix this test - add_friend method doesn't exist
    // #[test]
    // fn test_are_friends_bidirectional_check() {
    //     let (_db, repo) = setup_test_db();
    //     let user1 = Uuid::new_v4();
    //     let user2 = Uuid::new_v4();

    //     // Add friendship from user1 to user2
    //     repo.add_friend(&user1, &user2).unwrap();

    //     // Check friendship exists from user1's perspective
    //     assert!(repo.are_friends(&user1, &user2).unwrap());

    //     // Check friendship does NOT exist from user2's perspective (one-way)
    //     assert!(!repo.are_friends(&user2, &user1).unwrap());
    // }
}
