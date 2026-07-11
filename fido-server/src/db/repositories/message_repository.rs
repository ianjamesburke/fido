
use anyhow::{Context, Result};
use rusqlite::params;
use uuid::Uuid;

use fido_types::Message;

use crate::db::{row, DbPool};

#[derive(Clone)]
pub struct MessageRepository {
    pool: DbPool,
}

impl MessageRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new message
    pub fn create(&self, message: &Message) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO messages (id, channel_id, author_id, content, created_at) VALUES (?, ?, ?, ?, ?)",
            (
                message.id.to_string(),
                message.channel_id.to_string(),
                message.author_id.to_string(),
                &message.content,
                message.created_at.to_rfc3339(),
            ),
        )
        .context("Failed to create message")?;
        Ok(())
    }

    /// List messages in a channel with cursor pagination on (channel_id, created_at, id).
    ///
    /// Cursors are message ids, but ordering is by creation time with id as a tiebreaker.
    /// Results are returned oldest-to-newest.
    pub fn list(
        &self,
        channel_id: &Uuid,
        before: Option<&Uuid>,
        after: Option<&Uuid>,
        limit: i32,
    ) -> Result<Vec<Message>> {
        let conn = self.pool.get()?;
        let channel_id_str = channel_id.to_string();

        let mut messages = if let Some(after) = after {
            let mut stmt = conn.prepare(
                "SELECT id, channel_id, author_id, content, created_at FROM messages
                 WHERE channel_id = ?
                   AND (
                     created_at > (SELECT created_at FROM messages WHERE id = ? AND channel_id = ?)
                     OR (
                       created_at = (SELECT created_at FROM messages WHERE id = ? AND channel_id = ?)
                       AND id > ?
                     )
                   )
                 ORDER BY created_at ASC, id ASC
                 LIMIT ?",
            )?;
            let rows = stmt
                .query_map(
                    params![
                        channel_id_str,
                        after.to_string(),
                        channel_id_str,
                        after.to_string(),
                        channel_id_str,
                        after.to_string(),
                        limit
                    ],
                    map_message_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        } else if let Some(before) = before {
            let mut stmt = conn.prepare(
                "SELECT id, channel_id, author_id, content, created_at FROM messages
                 WHERE channel_id = ?
                   AND (
                     created_at < (SELECT created_at FROM messages WHERE id = ? AND channel_id = ?)
                     OR (
                       created_at = (SELECT created_at FROM messages WHERE id = ? AND channel_id = ?)
                       AND id < ?
                     )
                   )
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?",
            )?;
            let mut rows = stmt
                .query_map(
                    params![
                        channel_id_str,
                        before.to_string(),
                        channel_id_str,
                        before.to_string(),
                        channel_id_str,
                        before.to_string(),
                        limit
                    ],
                    map_message_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            rows.reverse();
            rows
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, channel_id, author_id, content, created_at FROM messages
                 WHERE channel_id = ? ORDER BY created_at DESC, id DESC LIMIT ?",
            )?;
            let mut rows = stmt
                .query_map(params![channel_id_str, limit], map_message_row)?
                .collect::<Result<Vec<_>, _>>()?;
            rows.reverse();
            rows
        };

        messages.shrink_to_fit();
        Ok(messages)
    }
}

fn map_message_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    let id_str: String = row.get(0)?;
    let channel_id_str: String = row.get(1)?;
    let author_id_str: String = row.get(2)?;
    let created_at_str: String = row.get(4)?;

    Ok(Message {
        id: row::parse_uuid(&id_str, 0)?,
        channel_id: row::parse_uuid(&channel_id_str, 1)?,
        author_id: row::parse_uuid(&author_id_str, 2)?,
        content: row.get(3)?,
        created_at: row::parse_datetime(&created_at_str, 4)?,
    })
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;
    use chrono::{TimeZone, Utc};

    fn setup() -> Result<(Database, Uuid, Uuid)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let conn = db.pool.get()?;

        let user_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user) VALUES (?, ?, ?, ?)",
            (user_id.to_string(), "author1", "2024-01-01T00:00:00Z", 1),
        )?;

        let community_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO communities (id, github_repo_id, owner, name, require_thread_approval, created_at)
             VALUES (?, ?, ?, ?, 0, ?)",
            (community_id.to_string(), 1_i64, "octocat", "hello", "2024-01-01T00:00:00Z"),
        )?;

        let channel_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO channels (id, community_id, name, created_at) VALUES (?, ?, ?, ?)",
            (
                channel_id.to_string(),
                community_id.to_string(),
                "general",
                "2024-01-01T00:00:00Z",
            ),
        )?;

        Ok((db, channel_id, user_id))
    }

    fn make_message(
        channel_id: Uuid,
        author_id: Uuid,
        id: Uuid,
        content: &str,
        second: u32,
    ) -> Message {
        Message {
            id,
            channel_id,
            author_id,
            content: content.to_string(),
            created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, second).unwrap(),
        }
    }

    #[test]
    fn test_create_and_list_default() -> Result<()> {
        let (db, channel_id, user_id) = setup()?;
        let repo = MessageRepository::new(db.pool);

        // ids intentionally do not match chronological order
        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let c = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        repo.create(&make_message(channel_id, user_id, a, "first", 1))?;
        repo.create(&make_message(channel_id, user_id, b, "second", 2))?;
        repo.create(&make_message(channel_id, user_id, c, "third", 3))?;

        let all = repo.list(&channel_id, None, None, 10)?;
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].content, "first");
        assert_eq!(all[2].content, "third");
        Ok(())
    }

    #[test]
    fn test_cursor_pagination() -> Result<()> {
        let (db, channel_id, user_id) = setup()?;
        let repo = MessageRepository::new(db.pool);

        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let c = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        repo.create(&make_message(channel_id, user_id, a, "first", 1))?;
        repo.create(&make_message(channel_id, user_id, b, "second", 2))?;
        repo.create(&make_message(channel_id, user_id, c, "third", 3))?;

        let after = repo.list(&channel_id, None, Some(&a), 10)?;
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].content, "second");

        let before = repo.list(&channel_id, Some(&c), None, 1)?;
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].content, "second");
        Ok(())
    }
}
