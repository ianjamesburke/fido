use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use fido_types::{Post, SortOrder};

use crate::db::{row, DbPool};

const POST_SELECT_WITH_REPLY_COUNT: &str = "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id,
                    (SELECT COUNT(*) FROM posts WHERE parent_post_id = p.id) as reply_count,
                    p.reply_to_user_id, u2.username as reply_to_username, p.community_id, p.approved
             FROM posts p
             JOIN users u ON p.author_id = u.id
             LEFT JOIN users u2 ON p.reply_to_user_id = u2.id";

const PENDING_POSTS_QUERY: &str = "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id,
                    (SELECT COUNT(*) FROM posts WHERE parent_post_id = p.id) as reply_count,
                    p.reply_to_user_id, u2.username as reply_to_username, p.community_id, p.approved
             FROM posts p
             JOIN users u ON p.author_id = u.id
             LEFT JOIN users u2 ON p.reply_to_user_id = u2.id
             WHERE p.community_id = ? AND p.parent_post_id IS NULL AND p.approved = 0
             ORDER BY p.created_at ASC
             LIMIT ?";

#[derive(Clone)]
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
            "INSERT INTO posts (id, author_id, community_id, content, created_at, upvotes, downvotes, approved, parent_post_id, reply_to_user_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                post.id.to_string(),
                post.author_id.to_string(),
                post.community_id.to_string(),
                &post.content,
                post.created_at.to_rfc3339(),
                post.upvotes,
                post.downvotes,
                if post.approved { 1 } else { 0 },
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
    pub fn get_posts(
        &self,
        community_id: &Uuid,
        sort_order: SortOrder,
        limit: i32,
    ) -> Result<Vec<Post>> {
        let conn = self.pool.get()?;

        let query = feed_posts_query(sort_order);

        let mut stmt = conn.prepare(&query)?;

        let posts = stmt
            .query_map(params![community_id.to_string(), limit], map_post_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(posts)
    }

    /// Get pending top-level posts for admin approval.
    pub fn get_pending_posts(&self, community_id: &Uuid, limit: i32) -> Result<Vec<Post>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(PENDING_POSTS_QUERY)?;

        let posts = stmt
            .query_map(params![community_id.to_string(), limit], map_post_row)?
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
                    p.reply_to_user_id, u2.username as reply_to_username, p.community_id, p.approved
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
                    p.reply_to_user_id, u2.username as reply_to_username, p.community_id, p.approved
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

    /// Mark a top-level thread as approved.
    pub fn approve(&self, post_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE posts SET approved = 1 WHERE id = ? AND parent_post_id IS NULL",
            [post_id.to_string()],
        )
        .context("Failed to approve post")?;
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

    /// Fetch all replies for a given post_id (recursively, maintaining tree structure)
    pub fn get_replies(&self, parent_post_id: &Uuid) -> Result<Vec<Post>> {
        let conn = self.pool.get()?;

        // Use recursive CTE to fetch entire reply tree
        let mut stmt = conn.prepare(
            "WITH RECURSIVE reply_tree AS (
                -- Base case: direct replies to the parent post
                SELECT p.id, p.author_id, p.content, p.created_at, p.upvotes, p.downvotes,
                       p.parent_post_id, p.reply_to_user_id, p.community_id, p.approved, 0 as depth
                FROM posts p
                WHERE p.parent_post_id = ?

                UNION ALL

                -- Recursive case: replies to replies
                SELECT p.id, p.author_id, p.content, p.created_at, p.upvotes, p.downvotes,
                       p.parent_post_id, p.reply_to_user_id, p.community_id, p.approved, rt.depth + 1
                FROM posts p
                INNER JOIN reply_tree rt ON p.parent_post_id = rt.id
            )
            SELECT rt.id, rt.author_id, u.username, rt.content, rt.created_at,
                   rt.upvotes, rt.downvotes, rt.parent_post_id,
                   (SELECT COUNT(*) FROM posts WHERE parent_post_id = rt.id) as reply_count,
                   rt.reply_to_user_id, u2.username as reply_to_username, rt.community_id, rt.approved, rt.depth
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
        community_id: &Uuid,
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
                    p.reply_to_user_id, u2.username as reply_to_username, p.community_id, p.approved
             FROM posts p
             JOIN users u ON p.author_id = u.id
             LEFT JOIN users u2 ON p.reply_to_user_id = u2.id
             WHERE p.community_id = ? AND LOWER(u.username) = LOWER(?) AND p.parent_post_id IS NULL AND p.approved = 1
             {}
             LIMIT ?",
            order_clause
        );

        let mut stmt = conn.prepare(&query)?;

        let posts = stmt
            .query_map(
                params![community_id.to_string(), username, limit],
                map_post_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(posts)
    }
}

fn feed_posts_query(sort_order: SortOrder) -> String {
    // Safe: order_clause is built from whitelisted enum values only.
    let order_clause = match sort_order {
        SortOrder::Newest => "ORDER BY p.created_at DESC",
        SortOrder::Popular => "ORDER BY p.upvotes DESC, p.created_at DESC",
        SortOrder::Controversial => "ORDER BY ABS(p.upvotes - p.downvotes) ASC, p.created_at DESC",
    };

    format!(
        "{POST_SELECT_WITH_REPLY_COUNT}
             WHERE p.community_id = ? AND p.parent_post_id IS NULL AND p.approved = 1
             {order_clause}
             LIMIT ?"
    )
}

fn map_post_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Post> {
    let id_str: String = row.get(0)?;
    let author_id_str: String = row.get(1)?;
    let created_at_str: String = row.get(4)?;
    let parent_post_id_str: Option<String> = row.get(7)?;
    let reply_to_user_id_str: Option<String> = row.get(9)?;
    let community_id_str: String = row.get(11)?;

    Ok(Post {
        id: row::parse_uuid(&id_str, 0)?,
        author_id: row::parse_uuid(&author_id_str, 1)?,
        author_username: row.get(2)?,
        community_id: row::parse_uuid(&community_id_str, 11)?,
        content: row.get(3)?,
        created_at: row::parse_datetime(&created_at_str, 4)?,
        upvotes: row.get(5)?,
        downvotes: row.get(6)?,
        approved: row.get::<_, i32>(12)? == 1,
        hashtags: Vec::new(),
        user_vote: None, // Will be populated by API layer if user is authenticated
        parent_post_id: row::parse_optional_uuid(parent_post_id_str.as_deref(), 7)?,
        reply_count: row.get(8)?,
        reply_to_user_id: row::parse_optional_uuid(reply_to_user_id_str.as_deref(), 9)?,
        reply_to_username: row.get(10)?,
    })
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;
    use chrono::{Duration, Utc};
    use std::time::Instant;

    const PERF_THREAD_COUNT: usize = 5_000;

    fn setup_large_feed() -> Result<(Database, PostRepository, Uuid)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let conn = db.connection()?;

        let user_id = Uuid::new_v4();
        let community_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user)
             VALUES (?, 'feed-user', ?, 1)",
            (user_id.to_string(), Utc::now().to_rfc3339()),
        )?;
        conn.execute(
            "INSERT INTO communities (id, github_repo_id, owner, name, require_thread_approval, created_at)
             VALUES (?, 4242, 'octocat', 'feed', 0, ?)",
            (community_id.to_string(), Utc::now().to_rfc3339()),
        )?;

        let tx = conn.unchecked_transaction()?;
        {
            let mut post_stmt = tx.prepare(
                "INSERT INTO posts (id, author_id, community_id, content, created_at, upvotes, downvotes, approved, parent_post_id, reply_to_user_id)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL)",
            )?;
            let mut reply_stmt = tx.prepare(
                "INSERT INTO posts (id, author_id, community_id, content, created_at, upvotes, downvotes, approved, parent_post_id, reply_to_user_id)
                 VALUES (?, ?, ?, ?, ?, 0, 0, 1, ?, ?)",
            )?;

            for i in 0..PERF_THREAD_COUNT {
                let post_id = Uuid::new_v4();
                let created_at = Utc::now() - Duration::seconds(i as i64);
                let approved = if i % 17 == 0 { 0 } else { 1 };
                post_stmt.execute((
                    post_id.to_string(),
                    user_id.to_string(),
                    community_id.to_string(),
                    format!("thread {i}"),
                    created_at.to_rfc3339(),
                    (i % 500) as i32,
                    (i % 11) as i32,
                    approved,
                ))?;

                if i % 20 == 0 {
                    reply_stmt.execute((
                        Uuid::new_v4().to_string(),
                        user_id.to_string(),
                        community_id.to_string(),
                        format!("reply {i}"),
                        (created_at + Duration::milliseconds(1)).to_rfc3339(),
                        post_id.to_string(),
                        user_id.to_string(),
                    ))?;
                }
            }
        }
        tx.commit()?;
        drop(conn);

        Ok((
            db.clone(),
            PostRepository::new(db.pool.clone()),
            community_id,
        ))
    }

    fn explain_query_plan(
        conn: &rusqlite::Connection,
        query: &str,
        community_id: &Uuid,
    ) -> Result<Vec<String>> {
        let explain = format!("EXPLAIN QUERY PLAN {query}");
        let mut stmt = conn.prepare(&explain)?;
        let rows = stmt
            .query_map(params![community_id.to_string(), 50], |row| row.get(3))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok(rows)
    }

    fn assert_plan_uses(plan: &[String], index_name: &str) {
        assert!(
            plan.iter().any(|row| row.contains(index_name)),
            "expected plan to use {index_name}; plan:\n{}",
            plan.join("\n")
        );
    }

    fn assert_no_temp_sort(plan: &[String]) {
        assert!(
            plan.iter().all(|row| !row.contains("USE TEMP B-TREE")),
            "expected index-backed ordering; plan:\n{}",
            plan.join("\n")
        );
    }

    #[test]
    fn large_feed_queries_use_release_indexes() -> Result<()> {
        let (db, repo, community_id) = setup_large_feed()?;
        let conn = db.connection()?;

        for (sort, expected_index) in [
            (SortOrder::Newest, "idx_posts_feed_newest"),
            (SortOrder::Popular, "idx_posts_feed_popular"),
            (SortOrder::Controversial, "idx_posts_feed_controversial"),
        ] {
            let started = Instant::now();
            let posts = repo.get_posts(&community_id, sort, 50)?;
            let elapsed = started.elapsed();
            eprintln!(
                "feed query {:?}: {} rows from {} threads in {:?}",
                sort,
                posts.len(),
                PERF_THREAD_COUNT,
                elapsed
            );
            assert_eq!(posts.len(), 50);

            let plan = explain_query_plan(&conn, &feed_posts_query(sort), &community_id)?;
            assert_plan_uses(&plan, expected_index);
            assert_plan_uses(&plan, "idx_posts_parent_post_id");
            assert_no_temp_sort(&plan);
        }

        let started = Instant::now();
        let pending = repo.get_pending_posts(&community_id, 50)?;
        eprintln!(
            "pending feed query: {} rows from {} threads in {:?}",
            pending.len(),
            PERF_THREAD_COUNT,
            started.elapsed()
        );
        assert!(!pending.is_empty());

        let pending_plan = explain_query_plan(&conn, PENDING_POSTS_QUERY, &community_id)?;
        assert_plan_uses(&pending_plan, "idx_posts_feed_newest");
        assert_plan_uses(&pending_plan, "idx_posts_parent_post_id");
        assert_no_temp_sort(&pending_plan);

        Ok(())
    }
}
