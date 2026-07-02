use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use fido_types::DirectMessage;

use crate::db::{row, DbPool};

/// Summary of a direct message conversation, without the full message history.
#[derive(Debug, Clone)]
pub struct DirectMessageConversationSummary {
    pub other_user_id: Uuid,
    pub last_message: String,
    pub last_message_time: DateTime<Utc>,
    pub unread_count: usize,
}

#[derive(Clone)]
pub struct DirectMessageRepository {
    pool: DbPool,
}

impl DirectMessageRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new direct message
    pub fn create(&self, dm: &DirectMessage) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO direct_messages (id, from_user_id, to_user_id, content, created_at, is_read, deleted_by_from_user, deleted_by_to_user) 
             VALUES (?, ?, ?, ?, ?, ?, 0, 0)",
            (
                dm.id.to_string(),
                dm.from_user_id.to_string(),
                dm.to_user_id.to_string(),
                &dm.content,
                dm.created_at.to_rfc3339(),
                if dm.is_read { 1 } else { 0 },
            ),
        ).context("Failed to create direct message")?;
        Ok(())
    }

    /// Get conversation between two users (excluding messages deleted by the requesting user)
    pub fn get_conversation(&self, user1_id: &Uuid, user2_id: &Uuid) -> Result<Vec<DirectMessage>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, from_user_id, to_user_id, content, created_at, is_read
             FROM direct_messages
             WHERE ((from_user_id = ? AND to_user_id = ?) OR (from_user_id = ? AND to_user_id = ?))
               AND ((from_user_id = ? AND deleted_by_from_user = 0) OR (to_user_id = ? AND deleted_by_to_user = 0))
             ORDER BY created_at ASC"
        )?;

        let messages = stmt
            .query_map(
                (
                    user1_id.to_string(),
                    user2_id.to_string(),
                    user2_id.to_string(),
                    user1_id.to_string(),
                    user1_id.to_string(),
                    user1_id.to_string(),
                ),
                |row| {
                    let id_str: String = row.get(0)?;
                    let from_id_str: String = row.get(1)?;
                    let to_id_str: String = row.get(2)?;
                    let created_at_str: String = row.get(4)?;

                    Ok(DirectMessage {
                        id: row::parse_uuid(&id_str, 0)?,
                        from_user_id: row::parse_uuid(&from_id_str, 1)?,
                        to_user_id: row::parse_uuid(&to_id_str, 2)?,
                        from_username: String::new(), // Will be populated by API layer
                        to_username: String::new(),   // Will be populated by API layer
                        content: row.get(3)?,
                        created_at: row::parse_datetime(&created_at_str, 4)?,
                        is_read: row.get::<_, i32>(5)? == 1,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// Count all messages exchanged by two users, including messages hidden by either user.
    pub fn count_between(&self, user1_id: &Uuid, user2_id: &Uuid) -> Result<i64> {
        let conn = self.pool.get()?;
        let count = conn.query_row(
            "SELECT COUNT(*)
             FROM direct_messages
             WHERE (from_user_id = ? AND to_user_id = ?)
                OR (from_user_id = ? AND to_user_id = ?)",
            (
                user1_id.to_string(),
                user2_id.to_string(),
                user2_id.to_string(),
                user1_id.to_string(),
            ),
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get conversation summaries without loading full message histories.
    pub fn get_conversation_summaries(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<DirectMessageConversationSummary>> {
        let conn = self.pool.get()?;
        let user_id_str = user_id.to_string();
        let mut stmt = conn.prepare(
            "WITH visible_messages AS (
                SELECT
                    CASE
                        WHEN from_user_id = ? THEN to_user_id
                        ELSE from_user_id
                    END AS other_user_id,
                    content,
                    created_at,
                    CASE
                        WHEN to_user_id = ? AND is_read = 0 THEN 1
                        ELSE 0
                    END AS unread
                FROM direct_messages
                WHERE (from_user_id = ? AND deleted_by_from_user = 0)
                   OR (to_user_id = ? AND deleted_by_to_user = 0)
             ),
             ranked_messages AS (
                SELECT
                    other_user_id,
                    content,
                    created_at,
                    unread,
                    ROW_NUMBER() OVER (
                        PARTITION BY other_user_id
                        ORDER BY created_at DESC
                    ) AS recency_rank
                FROM visible_messages
             )
             SELECT
                other_user_id,
                MAX(CASE WHEN recency_rank = 1 THEN content END) AS last_message,
                MAX(CASE WHEN recency_rank = 1 THEN created_at END) AS last_message_time,
                SUM(unread) AS unread_count
             FROM ranked_messages
             GROUP BY other_user_id
             ORDER BY last_message_time DESC",
        )?;

        let summaries = stmt
            .query_map(
                (&user_id_str, &user_id_str, &user_id_str, &user_id_str),
                |row| {
                    let other_user_id: String = row.get(0)?;
                    let last_message_time: String = row.get(2)?;
                    Ok(DirectMessageConversationSummary {
                        other_user_id: row::parse_uuid(&other_user_id, 0)?,
                        last_message: row.get(1)?,
                        last_message_time: row::parse_datetime(&last_message_time, 2)?,
                        unread_count: row.get::<_, i64>(3)? as usize,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(summaries)
    }

    /// Mark messages as read (only non-deleted messages)
    pub fn mark_as_read(&self, user_id: &Uuid, other_user_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE direct_messages 
             SET is_read = 1 
             WHERE to_user_id = ? AND from_user_id = ? AND deleted_by_to_user = 0",
            (user_id.to_string(), other_user_id.to_string()),
        )
        .context("Failed to mark messages as read")?;
        Ok(())
    }

    /// Get unread message count for a user (excluding deleted messages)
    #[allow(dead_code)]
    pub fn get_unread_count(&self, user_id: &Uuid) -> Result<i32> {
        let conn = self.pool.get()?;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM direct_messages 
             WHERE to_user_id = ? AND is_read = 0 AND deleted_by_to_user = 0",
            [user_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Delete conversation for a specific user (soft delete - hides from their view only)
    pub fn delete_conversation(&self, user_id: &Uuid, other_user_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;

        // Mark messages as deleted for this user only
        // For messages where user is the sender: set deleted_by_from_user = 1
        conn.execute(
            "UPDATE direct_messages 
             SET deleted_by_from_user = 1
             WHERE from_user_id = ? AND to_user_id = ?",
            (user_id.to_string(), other_user_id.to_string()),
        )
        .context("Failed to mark sent messages as deleted")?;

        // For messages where user is the receiver: set deleted_by_to_user = 1
        conn.execute(
            "UPDATE direct_messages 
             SET deleted_by_to_user = 1
             WHERE to_user_id = ? AND from_user_id = ?",
            (user_id.to_string(), other_user_id.to_string()),
        )
        .context("Failed to mark received messages as deleted")?;

        Ok(())
    }

    /// Undelete conversation for a specific user (when they send a new message)
    #[allow(dead_code)]
    pub fn undelete_conversation(&self, user_id: &Uuid, other_user_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;

        // Unmark messages as deleted for this user
        conn.execute(
            "UPDATE direct_messages 
             SET deleted_by_from_user = 0
             WHERE from_user_id = ? AND to_user_id = ?",
            (user_id.to_string(), other_user_id.to_string()),
        )
        .context("Failed to unmark sent messages as deleted")?;

        conn.execute(
            "UPDATE direct_messages 
             SET deleted_by_to_user = 0
             WHERE from_user_id = ? AND to_user_id = ?",
            (other_user_id.to_string(), user_id.to_string()),
        )
        .context("Failed to unmark received messages as deleted")?;

        Ok(())
    }
}
