use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use fido_types::{Post, SortOrder};

use crate::db::{row, DbPool};

pub struct PostRepository {
    pool: DbPool,
}

impl PostRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new post
    pub fn create(&self, post: &Post) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO posts (id, author_id, content, created_at, upvotes, downvotes, parent_post_id, reply_to_user_id) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                post.id.to_string(),
                post.author_id.to_string(),
                &post.content,
                post.created_at.to_rfc3339(),
                post.upvotes,
                post.downvotes,
                post.parent_post_id.map(|id| id.to_string()),
                post.reply_to_user_id.map(|id| id.to_string()),
            ),
        ).context("Failed to create post")?;
        Ok(())
    }

    /// Update post content by ID
    pub fn update_content(&self, post_id: &Uuid, content: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE posts SET content = ? WHERE id = ?",
            (content, post_id.to_string()),
        )
        .context("Failed to update post content")?;
        Ok(())
    }

    /// Delete a post by ID (replies and related rows cascade via FK)
    pub fn delete(&self, post_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM posts WHERE id = ?", [post_id.to_string()])
            .context("Failed to delete post")?;
        Ok(())
    }

    /// Get posts with sorting and limit
    ///
    /// # SQL Injection Safety
    /// This method uses format!() to build the ORDER BY clause, but it is safe because:
    /// - The `sort_order` parameter is a validated enum (SortOrder), not user input
    /// - The enum can only have three values: Newest, Popular, Controversial
    /// - Each enum value maps to a hardcoded SQL clause
    /// - The API layer validates sort order before calling this method
    /// - All other parameters (limit) use parameterized queries
    pub fn get_posts(&self, sort_order: SortOrder, limit: i32) -> Result<Vec<Post>> {
        let conn = self.pool.get()?;

        // Safe: order_clause is built from whitelisted enum values only
        let order_clause = match sort_order {
            SortOrder::Newest => "ORDER BY p.created_at DESC",
            SortOrder::Popular => "ORDER BY p.upvotes DESC, p.created_at DESC",
            SortOrder::Controversial => {
                "ORDER BY ABS(p.upvotes - p.downvotes) ASC, p.created_at DESC"
            }
        };

        let query = format!(
            "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id,
                    (SELECT COUNT(*) FROM posts WHERE parent_post_id = p.id) as reply_count,
                    p.reply_to_user_id, u2.username as reply_to_username
             FROM posts p
             JOIN users u ON p.author_id = u.id
             LEFT JOIN users u2 ON p.reply_to_user_id = u2.id
             WHERE p.parent_post_id IS NULL
             {}
             LIMIT ?",
            order_clause
        );

        let mut stmt = conn.prepare(&query)?;

        let posts = stmt
            .query_map([limit], map_post_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(posts)
    }

    /// Get posts by a specific user
    #[allow(dead_code)]
    pub fn get_by_user(&self, user_id: &Uuid) -> Result<Vec<Post>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id,
                    (SELECT COUNT(*) FROM posts WHERE parent_post_id = p.id) as reply_count,
                    p.reply_to_user_id, u2.username as reply_to_username
             FROM posts p
             JOIN users u ON p.author_id = u.id
             LEFT JOIN users u2 ON p.reply_to_user_id = u2.id
             WHERE p.author_id = ?
             ORDER BY p.created_at DESC"
        )?;

        let posts = stmt
            .query_map([user_id.to_string()], map_post_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(posts)
    }

    /// Get a single post by ID
    pub fn get_by_id(&self, post_id: &Uuid) -> Result<Option<Post>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id,
                    (SELECT COUNT(*) FROM posts WHERE parent_post_id = p.id) as reply_count,
                    p.reply_to_user_id, u2.username as reply_to_username
             FROM posts p
             JOIN users u ON p.author_id = u.id
             LEFT JOIN users u2 ON p.reply_to_user_id = u2.id
             WHERE p.id = ?"
        )?;

        let post = stmt
            .query_row([post_id.to_string()], map_post_row)
            .optional()?;

        Ok(post)
    }

    /// Update vote counts for a post
    pub fn update_vote_counts(&self, post_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;

        // Recalculate vote counts from votes table
        conn.execute(
            "UPDATE posts 
             SET upvotes = (SELECT COUNT(*) FROM votes WHERE post_id = ? AND direction = 'up'),
                 downvotes = (SELECT COUNT(*) FROM votes WHERE post_id = ? AND direction = 'down')
             WHERE id = ?",
            (
                post_id.to_string(),
                post_id.to_string(),
                post_id.to_string(),
            ),
        )
        .context("Failed to update vote counts")?;

        Ok(())
    }

    /// Get post count for a user
    pub fn get_post_count(&self, user_id: &Uuid) -> Result<i32> {
        let conn = self.pool.get()?;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM posts WHERE author_id = ?",
            [user_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Extract hashtags from post content using regex
    #[allow(dead_code)]
    pub fn extract_hashtags(content: &str) -> Vec<String> {
        let mut hashtags = Vec::new();
        for word in content.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 {
                // Remove the # and any trailing punctuation
                let tag = word[1..].trim_end_matches(|c: char| !c.is_alphanumeric());
                if !tag.is_empty() {
                    hashtags.push(tag.to_lowercase());
                }
            }
        }
        hashtags
    }

    /// Fetch all replies for a given post_id (recursively, maintaining tree structure)
    pub fn get_replies(&self, parent_post_id: &Uuid) -> Result<Vec<Post>> {
        let conn = self.pool.get()?;

        // Use recursive CTE to fetch entire reply tree
        let mut stmt = conn.prepare(
            "WITH RECURSIVE reply_tree AS (
                -- Base case: direct replies to the parent post
                SELECT p.id, p.author_id, p.content, p.created_at, p.upvotes, p.downvotes, 
                       p.parent_post_id, p.reply_to_user_id, 0 as depth
                FROM posts p
                WHERE p.parent_post_id = ?
                
                UNION ALL
                
                -- Recursive case: replies to replies
                SELECT p.id, p.author_id, p.content, p.created_at, p.upvotes, p.downvotes,
                       p.parent_post_id, p.reply_to_user_id, rt.depth + 1
                FROM posts p
                INNER JOIN reply_tree rt ON p.parent_post_id = rt.id
            )
            SELECT rt.id, rt.author_id, u.username, rt.content, rt.created_at, 
                   rt.upvotes, rt.downvotes, rt.parent_post_id,
                   (SELECT COUNT(*) FROM posts WHERE parent_post_id = rt.id) as reply_count,
                   rt.reply_to_user_id, u2.username as reply_to_username, rt.depth
            FROM reply_tree rt
            JOIN users u ON rt.author_id = u.id
            LEFT JOIN users u2 ON rt.reply_to_user_id = u2.id
            ORDER BY rt.depth ASC, rt.created_at ASC",
        )?;

        let replies = stmt
            .query_map([parent_post_id.to_string()], map_post_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(replies)
    }

    /// Check if a post has replies
    #[allow(dead_code)]
    pub fn has_replies(&self, post_id: &Uuid) -> Result<bool> {
        let conn = self.pool.get()?;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM posts WHERE parent_post_id = ?",
            [post_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get posts filtered by hashtag
    ///
    /// # SQL Injection Safety
    /// This method uses format!() to build the ORDER BY clause, but it is safe because:
    /// - The `sort_order` parameter is a validated enum (SortOrder), not user input
    /// - The enum can only have three values: Newest, Popular, Controversial
    /// - Each enum value maps to a hardcoded SQL clause
    /// - The API layer validates sort order before calling this method
    /// - All other parameters (hashtag_name, limit) use parameterized queries
    pub fn get_posts_by_hashtag(
        &self,
        hashtag_name: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> Result<Vec<Post>> {
        let conn = self.pool.get()?;

        // Safe: order_clause is built from whitelisted enum values only
        let order_clause = match sort_order {
            SortOrder::Newest => "ORDER BY p.created_at DESC",
            SortOrder::Popular => "ORDER BY p.upvotes DESC, p.created_at DESC",
            SortOrder::Controversial => {
                "ORDER BY ABS(p.upvotes - p.downvotes) ASC, p.created_at DESC"
            }
        };

        let query = format!(
            "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id,
                    (SELECT COUNT(*) FROM posts WHERE parent_post_id = p.id) as reply_count,
                    p.reply_to_user_id, u2.username as reply_to_username
             FROM posts p
             JOIN users u ON p.author_id = u.id
             LEFT JOIN users u2 ON p.reply_to_user_id = u2.id
             JOIN post_hashtags ph ON p.id = ph.post_id
             JOIN hashtags h ON ph.hashtag_id = h.id
             WHERE LOWER(h.name) = LOWER(?) AND p.parent_post_id IS NULL
             {}
             LIMIT ?",
            order_clause
        );

        let mut stmt = conn.prepare(&query)?;

        let posts = stmt
            .query_map(params![hashtag_name, limit], map_post_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(posts)
    }

    /// Get posts filtered by username
    ///
    /// # SQL Injection Safety
    /// This method uses format!() to build the ORDER BY clause, but it is safe because:
    /// - The `sort_order` parameter is a validated enum (SortOrder), not user input
    /// - The enum can only have three values: Newest, Popular, Controversial
    /// - Each enum value maps to a hardcoded SQL clause
    /// - The API layer validates sort order before calling this method
    /// - All other parameters (username, limit) use parameterized queries
    pub fn get_posts_by_username(
        &self,
        username: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> Result<Vec<Post>> {
        let conn = self.pool.get()?;

        // Safe: order_clause is built from whitelisted enum values only
        let order_clause = match sort_order {
            SortOrder::Newest => "ORDER BY p.created_at DESC",
            SortOrder::Popular => "ORDER BY p.upvotes DESC, p.created_at DESC",
            SortOrder::Controversial => {
                "ORDER BY ABS(p.upvotes - p.downvotes) ASC, p.created_at DESC"
            }
        };

        let query = format!(
            "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id,
                    (SELECT COUNT(*) FROM posts WHERE parent_post_id = p.id) as reply_count,
                    p.reply_to_user_id, u2.username as reply_to_username
             FROM posts p
             JOIN users u ON p.author_id = u.id
             LEFT JOIN users u2 ON p.reply_to_user_id = u2.id
             WHERE LOWER(u.username) = LOWER(?) AND p.parent_post_id IS NULL
             {}
             LIMIT ?",
            order_clause
        );

        let mut stmt = conn.prepare(&query)?;

        let posts = stmt
            .query_map(params![username, limit], map_post_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(posts)
    }

    /// Get posts filtered by both hashtag and username
    ///
    /// # SQL Injection Safety
    /// This method uses format!() to build the ORDER BY clause, but it is safe because:
    /// - The `sort_order` parameter is a validated enum (SortOrder), not user input
    /// - The enum can only have three values: Newest, Popular, Controversial
    /// - Each enum value maps to a hardcoded SQL clause
    /// - The API layer validates sort order before calling this method
    /// - All other parameters (hashtag_name, username, limit) use parameterized queries
    pub fn get_posts_by_hashtag_and_username(
        &self,
        hashtag_name: &str,
        username: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> Result<Vec<Post>> {
        let conn = self.pool.get()?;

        // Safe: order_clause is built from whitelisted enum values only
        let order_clause = match sort_order {
            SortOrder::Newest => "ORDER BY p.created_at DESC",
            SortOrder::Popular => "ORDER BY p.upvotes DESC, p.created_at DESC",
            SortOrder::Controversial => {
                "ORDER BY ABS(p.upvotes - p.downvotes) ASC, p.created_at DESC"
            }
        };

        let query = format!(
            "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id,
                    (SELECT COUNT(*) FROM posts WHERE parent_post_id = p.id) as reply_count,
                    p.reply_to_user_id, u2.username as reply_to_username
             FROM posts p
             JOIN users u ON p.author_id = u.id
             LEFT JOIN users u2 ON p.reply_to_user_id = u2.id
             JOIN post_hashtags ph ON p.id = ph.post_id
             JOIN hashtags h ON ph.hashtag_id = h.id
             WHERE LOWER(h.name) = LOWER(?) AND LOWER(u.username) = LOWER(?) AND p.parent_post_id IS NULL
             {}
             LIMIT ?",
            order_clause
        );

        let mut stmt = conn.prepare(&query)?;

        let posts = stmt
            .query_map(params![hashtag_name, username, limit], map_post_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(posts)
    }
}

fn map_post_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Post> {
    let id_str: String = row.get(0)?;
    let author_id_str: String = row.get(1)?;
    let created_at_str: String = row.get(4)?;
    let parent_post_id_str: Option<String> = row.get(7)?;
    let reply_to_user_id_str: Option<String> = row.get(9)?;

    Ok(Post {
        id: row::parse_uuid(&id_str, 0)?,
        author_id: row::parse_uuid(&author_id_str, 1)?,
        author_username: row.get(2)?,
        content: row.get(3)?,
        created_at: row::parse_datetime(&created_at_str, 4)?,
        upvotes: row.get(5)?,
        downvotes: row.get(6)?,
        hashtags: Vec::new(), // Will be populated separately
        user_vote: None,      // Will be populated by API layer if user is authenticated
        parent_post_id: row::parse_optional_uuid(parent_post_id_str.as_deref(), 7)?,
        reply_count: row.get(8)?,
        reply_to_user_id: row::parse_optional_uuid(reply_to_user_id_str.as_deref(), 9)?,
        reply_to_username: row.get(10)?,
    })
}
