use anyhow::{Context, Result};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::db::DbPool;

pub struct HashtagRepository {
    pool: DbPool,
}

impl HashtagRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Extract hashtags from text (matches #word pattern with letters, numbers, underscores)
    #[allow(dead_code)]
    pub fn extract_hashtags(text: &str) -> Vec<String> {
        let re = regex::Regex::new(r"#(\w+)").unwrap();
        re.captures_iter(text)
            .map(|cap| cap[1].to_lowercase())
            .collect()
    }

    /// Store hashtags for a post (creates hashtag entries if needed)
    pub fn store_hashtags(&self, post_id: &Uuid, hashtags: &[String]) -> Result<()> {
        let conn = self.pool.get()?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        for hashtag in hashtags {
            // Create hashtag if it doesn't exist
            let hashtag_id = Uuid::new_v4();
            conn.execute(
                "INSERT OR IGNORE INTO hashtags (id, name, created_at) VALUES (?, ?, ?)",
                (hashtag_id.to_string(), hashtag, now),
            )
            .context("Failed to create hashtag")?;

            // Get the hashtag ID (either just created or existing)
            let existing_id: String =
                conn.query_row("SELECT id FROM hashtags WHERE name = ?", [hashtag], |row| {
                    row.get(0)
                })?;

            // Link post to hashtag
            conn.execute(
                "INSERT OR IGNORE INTO post_hashtags (post_id, hashtag_id) VALUES (?, ?)",
                (post_id.to_string(), existing_id),
            )
            .context("Failed to link post to hashtag")?;
        }

        Ok(())
    }

    /// Get hashtags for a post
    pub fn get_by_post(&self, post_id: &Uuid) -> Result<Vec<String>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT h.name FROM hashtags h
             JOIN post_hashtags ph ON h.id = ph.hashtag_id
             WHERE ph.post_id = ?",
        )?;

        let hashtags = stmt
            .query_map([post_id.to_string()], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(hashtags)
    }

    /// Get user's followed hashtags
    pub fn get_followed_by_user(&self, user_id: &Uuid) -> Result<Vec<String>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT h.name FROM hashtags h
             JOIN user_hashtag_follows uhf ON h.id = uhf.hashtag_id
             WHERE uhf.user_id = ?
             ORDER BY uhf.followed_at DESC",
        )?;

        let hashtags = stmt
            .query_map([user_id.to_string()], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(hashtags)
    }

    /// Follow a hashtag (creates hashtag if it doesn't exist)
    pub fn follow_hashtag(&self, user_id: &Uuid, hashtag_name: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        // Create hashtag if it doesn't exist
        let hashtag_id = Uuid::new_v4();
        conn.execute(
            "INSERT OR IGNORE INTO hashtags (id, name, created_at) VALUES (?, ?, ?)",
            (hashtag_id.to_string(), hashtag_name.to_lowercase(), now),
        )
        .context("Failed to create hashtag")?;

        // Get the hashtag ID (either just created or existing)
        let existing_id: String = conn.query_row(
            "SELECT id FROM hashtags WHERE name = ?",
            [hashtag_name.to_lowercase()],
            |row| row.get(0),
        )?;

        // Follow the hashtag
        conn.execute(
            "INSERT OR IGNORE INTO user_hashtag_follows (user_id, hashtag_id, followed_at) VALUES (?, ?, ?)",
            (user_id.to_string(), existing_id, now),
        ).context("Failed to follow hashtag")?;

        Ok(())
    }

    /// Unfollow a hashtag
    pub fn unfollow_hashtag(&self, user_id: &Uuid, hashtag_name: &str) -> Result<()> {
        let conn = self.pool.get()?;

        conn.execute(
            "DELETE FROM user_hashtag_follows 
             WHERE user_id = ? AND hashtag_id = (SELECT id FROM hashtags WHERE name = ?)",
            (user_id.to_string(), hashtag_name),
        )
        .context("Failed to unfollow hashtag")?;

        Ok(())
    }

    /// Get most active hashtags for a user
    pub fn get_active_by_user(&self, user_id: &Uuid, limit: usize) -> Result<Vec<(String, i64)>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT h.name, uha.interaction_count 
             FROM hashtags h
             JOIN user_hashtag_activity uha ON h.id = uha.hashtag_id
             WHERE uha.user_id = ?
             ORDER BY uha.interaction_count DESC
             LIMIT ?",
        )?;

        let hashtags = stmt
            .query_map([user_id.to_string(), limit.to_string()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(hashtags)
    }

    /// Increment user hashtag activity
    pub fn increment_activity(&self, user_id: &Uuid, hashtag_name: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        // Get hashtag ID
        let hashtag_id: String = conn
            .query_row(
                "SELECT id FROM hashtags WHERE name = ?",
                [hashtag_name],
                |row| row.get(0),
            )
            .context("Hashtag not found")?;

        conn.execute(
            "INSERT INTO user_hashtag_activity (user_id, hashtag_id, interaction_count, last_interaction)
             VALUES (?, ?, 1, ?)
             ON CONFLICT(user_id, hashtag_id) DO UPDATE SET
                interaction_count = interaction_count + 1,
                last_interaction = ?",
            (user_id.to_string(), hashtag_id, now, now),
        ).context("Failed to update hashtag activity")?;

        Ok(())
    }

    /// Search hashtags by name
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let conn = self.pool.get()?;
        let search_pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT DISTINCT h.name 
             FROM hashtags h
             INNER JOIN post_hashtags ph ON h.id = ph.hashtag_id
             WHERE LOWER(h.name) LIKE LOWER(?)
             ORDER BY h.name
             LIMIT ?",
        )?;

        let hashtags = stmt
            .query_map([search_pattern, limit.to_string()], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(hashtags)
    }

    /// Delete hashtags for a post (when post is deleted)
    #[allow(dead_code)]
    pub fn delete_by_post(&self, post_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM post_hashtags WHERE post_id = ?",
            [post_id.to_string()],
        )
        .context("Failed to delete post hashtags")?;
        Ok(())
    }

    /// Get post count for a hashtag
    pub fn get_post_count(&self, hashtag_name: &str) -> Result<i32> {
        let conn = self.pool.get()?;
        let count: i32 = conn.query_row(
            "SELECT COUNT(DISTINCT ph.post_id) 
             FROM post_hashtags ph
             JOIN hashtags h ON ph.hashtag_id = h.id
             WHERE h.name = ?",
            [hashtag_name],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_test_db() -> Result<(Database, Uuid)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        // Create test user for foreign key constraints
        let user_id = Uuid::new_v4();
        let conn = db.pool.get()?;
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user) VALUES (?, ?, ?, ?)",
            (user_id.to_string(), "testuser1", "2024-01-01T00:00:00Z", 1),
        )?;
        Ok((db, user_id))
    }

    #[test]
    fn test_follow_and_get_hashtags() -> Result<()> {
        let (db, user_id) = setup_test_db()?;
        let repo = HashtagRepository::new(db.pool.clone());

        // Follow hashtags
        repo.follow_hashtag(&user_id, "rust")?;
        repo.follow_hashtag(&user_id, "programming")?;

        // Get hashtags back
        let retrieved = repo.get_followed_by_user(&user_id)?;
        assert_eq!(retrieved.len(), 2);
        assert!(retrieved.contains(&"rust".to_string()));
        assert!(retrieved.contains(&"programming".to_string()));

        Ok(())
    }

    #[test]
    fn test_follow_creates_hashtag() -> Result<()> {
        let (db, user_id) = setup_test_db()?;
        let repo = HashtagRepository::new(db.pool.clone());

        // Follow a new hashtag (which creates it)
        repo.follow_hashtag(&user_id, "newhashtag")?;

        // Verify it was created
        let followed = repo.get_followed_by_user(&user_id)?;
        assert_eq!(followed.len(), 1);
        assert_eq!(followed[0], "newhashtag");

        Ok(())
    }

    #[test]
    fn test_follow_unfollow() -> Result<()> {
        let (db, user_id) = setup_test_db()?;
        let repo = HashtagRepository::new(db.pool.clone());

        // Follow hashtag
        repo.follow_hashtag(&user_id, "rust")?;
        let followed = repo.get_followed_by_user(&user_id)?;
        assert_eq!(followed.len(), 1);

        // Unfollow hashtag
        repo.unfollow_hashtag(&user_id, "rust")?;
        let followed = repo.get_followed_by_user(&user_id)?;
        assert_eq!(followed.len(), 0);

        Ok(())
    }

    #[test]
    fn test_activity_increment() -> Result<()> {
        let (db, user_id) = setup_test_db()?;
        let repo = HashtagRepository::new(db.pool.clone());

        // Create hashtag first
        repo.follow_hashtag(&user_id, "rust")?;

        // Increment activity multiple times
        repo.increment_activity(&user_id, "rust")?;
        repo.increment_activity(&user_id, "rust")?;
        repo.increment_activity(&user_id, "rust")?;

        // Get active hashtags
        let active = repo.get_active_by_user(&user_id, 5)?;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].0, "rust");
        assert_eq!(active[0].1, 3); // 3 interactions

        Ok(())
    }

    #[test]
    fn test_search_hashtags() -> Result<()> {
        let (db, user_id) = setup_test_db()?;
        let repo = HashtagRepository::new(db.pool.clone());
        let conn = db.pool.get()?;

        // Create posts with hashtags (search requires hashtags to be associated with posts)
        let post1_id = Uuid::new_v4();
        let post2_id = Uuid::new_v4();
        let post3_id = Uuid::new_v4();

        // Insert posts into database
        conn.execute(
            "INSERT INTO posts (id, author_id, content, created_at) VALUES (?, ?, ?, ?)",
            (
                post1_id.to_string(),
                user_id.to_string(),
                "Post about #rust",
                "2024-01-01T00:00:00Z",
            ),
        )?;
        conn.execute(
            "INSERT INTO posts (id, author_id, content, created_at) VALUES (?, ?, ?, ?)",
            (
                post2_id.to_string(),
                user_id.to_string(),
                "Post about #rustlang",
                "2024-01-01T00:00:00Z",
            ),
        )?;
        conn.execute(
            "INSERT INTO posts (id, author_id, content, created_at) VALUES (?, ?, ?, ?)",
            (
                post3_id.to_string(),
                user_id.to_string(),
                "Post about #python",
                "2024-01-01T00:00:00Z",
            ),
        )?;

        // Store hashtags for posts
        repo.store_hashtags(&post1_id, &["rust".to_string()])?;
        repo.store_hashtags(&post2_id, &["rustlang".to_string()])?;
        repo.store_hashtags(&post3_id, &["python".to_string()])?;

        // Search for "rust"
        let results = repo.search("rust", 10)?;
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"rust".to_string()));
        assert!(results.contains(&"rustlang".to_string()));

        Ok(())
    }
}
