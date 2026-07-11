#![allow(dead_code)] // v2 repository is covered by sqlite-tests before its API surface is wired.

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::types::Type;
use rusqlite::OptionalExtension;
use uuid::Uuid;

use fido_types::{DmConversation, DmConversationState};

use crate::db::{row, DbPool};

#[derive(Clone)]
pub struct DmConversationRepository {
    pool: DbPool,
}

impl DmConversationRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get the conversation for a pair of users, in any argument order.
    pub fn get(&self, user_one: &Uuid, user_two: &Uuid) -> Result<Option<DmConversation>> {
        let (user_a, user_b) = ordered_pair(user_one, user_two);
        let conn = self.pool.get()?;
        let conversation = conn
            .query_row(
                "SELECT user_a, user_b, state, initiator_id, created_at FROM dm_conversations
                 WHERE user_a = ? AND user_b = ?",
                (user_a.to_string(), user_b.to_string()),
                map_conversation_row,
            )
            .optional()
            .context("Failed to fetch dm conversation")?;
        Ok(conversation)
    }

    /// List pending request conversations where `recipient_id` is not the initiator.
    pub fn list_pending_for_recipient(&self, recipient_id: &Uuid) -> Result<Vec<DmConversation>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT user_a, user_b, state, initiator_id, created_at FROM dm_conversations
             WHERE state = 'pending'
               AND (user_a = ? OR user_b = ?)
               AND initiator_id != ?
             ORDER BY created_at DESC",
        )?;
        let recipient = recipient_id.to_string();
        let conversations = stmt
            .query_map((&recipient, &recipient, &recipient), map_conversation_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(conversations)
    }

    /// Create a conversation for a pair of users, in any argument order.
    pub fn create(
        &self,
        user_one: &Uuid,
        user_two: &Uuid,
        initiator_id: &Uuid,
        state: DmConversationState,
    ) -> Result<DmConversation> {
        let (user_a, user_b) = ordered_pair(user_one, user_two);
        let created_at = Utc::now();
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO dm_conversations (user_a, user_b, state, initiator_id, created_at)
             VALUES (?, ?, ?, ?, ?)",
            (
                user_a.to_string(),
                user_b.to_string(),
                state.as_str(),
                initiator_id.to_string(),
                created_at.to_rfc3339(),
            ),
        )
        .context("Failed to create dm conversation")?;

        Ok(DmConversation {
            user_a,
            user_b,
            state,
            initiator_id: *initiator_id,
            created_at,
        })
    }

    /// Update the state of a conversation, in any argument order.
    pub fn set_state(
        &self,
        user_one: &Uuid,
        user_two: &Uuid,
        state: DmConversationState,
    ) -> Result<()> {
        let (user_a, user_b) = ordered_pair(user_one, user_two);
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE dm_conversations SET state = ? WHERE user_a = ? AND user_b = ?",
            (state.as_str(), user_a.to_string(), user_b.to_string()),
        )
        .context("Failed to update dm conversation state")?;
        Ok(())
    }
}

/// Return the pair ordered so that `user_a < user_b` lexicographically.
fn ordered_pair(first: &Uuid, second: &Uuid) -> (Uuid, Uuid) {
    if first.to_string() <= second.to_string() {
        (*first, *second)
    } else {
        (*second, *first)
    }
}

fn map_conversation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DmConversation> {
    let user_a_str: String = row.get(0)?;
    let user_b_str: String = row.get(1)?;
    let state_str: String = row.get(2)?;
    let initiator_id_str: String = row.get(3)?;
    let created_at_str: String = row.get(4)?;

    let state = DmConversationState::parse(&state_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid dm conversation state: {state_str}"),
            )),
        )
    })?;

    Ok(DmConversation {
        user_a: row::parse_uuid(&user_a_str, 0)?,
        user_b: row::parse_uuid(&user_b_str, 1)?,
        state,
        initiator_id: row::parse_uuid(&initiator_id_str, 3)?,
        created_at: row::parse_datetime(&created_at_str, 4)?,
    })
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup() -> Result<(Database, Uuid, Uuid)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let conn = db.pool.get()?;
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user) VALUES (?, ?, ?, 1), (?, ?, ?, 1)",
            (
                a.to_string(),
                "user_a",
                "2024-01-01T00:00:00Z",
                b.to_string(),
                "user_b",
                "2024-01-01T00:00:00Z",
            ),
        )?;
        Ok((db, a, b))
    }

    #[test]
    fn test_create_get_any_order() -> Result<()> {
        let (db, a, b) = setup()?;
        let repo = DmConversationRepository::new(db.pool);

        // Create with args in one order, fetch with the reverse order.
        let created = repo.create(&b, &a, &b, DmConversationState::Pending)?;
        assert!(created.user_a.to_string() < created.user_b.to_string());

        let fetched = repo.get(&a, &b)?.expect("conversation exists");
        assert_eq!(fetched.state, DmConversationState::Pending);
        assert_eq!(fetched.initiator_id, b);
        Ok(())
    }

    #[test]
    fn test_set_state() -> Result<()> {
        let (db, a, b) = setup()?;
        let repo = DmConversationRepository::new(db.pool);

        repo.create(&a, &b, &a, DmConversationState::Pending)?;
        repo.set_state(&b, &a, DmConversationState::Accepted)?;

        let fetched = repo.get(&a, &b)?.expect("conversation exists");
        assert_eq!(fetched.state, DmConversationState::Accepted);
        Ok(())
    }

    #[test]
    fn test_ordered_pair_invariant() {
        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let b = Uuid::parse_str("ffffffff-0000-0000-0000-000000000000").unwrap();
        let (x, y) = ordered_pair(&b, &a);
        assert_eq!(x, a);
        assert_eq!(y, b);
    }
}
