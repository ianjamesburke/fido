#![allow(dead_code)] // v2 repository is covered by sqlite-tests before its API surface is wired.

use anyhow::{Context, Result};
use rusqlite::params;
use rusqlite::types::Type;
use uuid::Uuid;

use fido_types::{Notification, NotificationType, NotificationUnreadCount};

use crate::db::{row, DbPool};

#[derive(Clone)]
pub struct NotificationRepository {
    pool: DbPool,
}

impl NotificationRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new notification
    pub fn create(&self, notification: &Notification) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO notifications (id, user_id, \"type\", actor_id, subject_type, subject_id, read, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                notification.id.to_string(),
                notification.user_id.to_string(),
                notification.notification_type.as_str(),
                notification.actor_id.to_string(),
                &notification.subject_type,
                &notification.subject_id,
                if notification.read { 1 } else { 0 },
                notification.created_at.to_rfc3339(),
            ),
        )
        .context("Failed to create notification")?;
        Ok(())
    }

    /// List notifications for a user, most recent first, paginated.
    pub fn list_for_user(
        &self,
        user_id: &Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Notification>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, \"type\", actor_id, subject_type, subject_id, read, created_at
             FROM notifications WHERE user_id = ?
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )?;
        let notifications = stmt
            .query_map(
                params![user_id.to_string(), limit, offset],
                map_notification_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(notifications)
    }

    /// Total unread notification count for a user
    pub fn unread_count(&self, user_id: &Uuid) -> Result<i64> {
        let conn = self.pool.get()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM notifications WHERE user_id = ? AND read = 0",
            [user_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Unread notification counts grouped by subject (for rail badges)
    pub fn unread_counts_by_subject(&self, user_id: &Uuid) -> Result<Vec<NotificationUnreadCount>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT subject_type, subject_id, COUNT(*) FROM notifications
             WHERE user_id = ? AND read = 0
             GROUP BY subject_type, subject_id",
        )?;
        let counts = stmt
            .query_map([user_id.to_string()], |row| {
                Ok(NotificationUnreadCount {
                    subject_type: row.get(0)?,
                    subject_id: row.get(1)?,
                    count: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(counts)
    }

    /// Mark a single notification as read for its owner.
    pub fn mark_read_for_user(&self, user_id: &Uuid, notification_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE notifications SET read = 1 WHERE id = ? AND user_id = ?",
            (notification_id.to_string(), user_id.to_string()),
        )
        .context("Failed to mark notification as read")?;
        Ok(())
    }

    /// Mark all of a user's notifications as read
    pub fn mark_all_read(&self, user_id: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE notifications SET read = 1 WHERE user_id = ?",
            [user_id.to_string()],
        )
        .context("Failed to mark all notifications as read")?;
        Ok(())
    }
}

fn map_notification_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Notification> {
    let id_str: String = row.get(0)?;
    let user_id_str: String = row.get(1)?;
    let type_str: String = row.get(2)?;
    let actor_id_str: String = row.get(3)?;
    let created_at_str: String = row.get(7)?;

    let notification_type = NotificationType::parse(&type_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid notification type: {type_str}"),
            )),
        )
    })?;

    Ok(Notification {
        id: row::parse_uuid(&id_str, 0)?,
        user_id: row::parse_uuid(&user_id_str, 1)?,
        notification_type,
        actor_id: row::parse_uuid(&actor_id_str, 3)?,
        subject_type: row.get(4)?,
        subject_id: row.get(5)?,
        read: row.get::<_, i32>(6)? == 1,
        created_at: row::parse_datetime(&created_at_str, 7)?,
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
        let recipient = Uuid::new_v4();
        let actor = Uuid::new_v4();
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user) VALUES (?, ?, ?, 1), (?, ?, ?, 1)",
            (
                recipient.to_string(),
                "recipient",
                "2024-01-01T00:00:00Z",
                actor.to_string(),
                "actor",
                "2024-01-01T00:00:00Z",
            ),
        )?;
        Ok((db, recipient, actor))
    }

    fn notification(user_id: Uuid, actor_id: Uuid, subject_type: &str) -> Notification {
        notification_with_subject_id(user_id, actor_id, subject_type, Uuid::new_v4().to_string())
    }

    fn notification_with_subject_id(
        user_id: Uuid,
        actor_id: Uuid,
        subject_type: &str,
        subject_id: String,
    ) -> Notification {
        Notification {
            id: Uuid::new_v4(),
            user_id,
            notification_type: NotificationType::Reply,
            actor_id,
            subject_type: subject_type.to_string(),
            subject_id,
            read: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_create_list_and_counts() -> Result<()> {
        let (db, recipient, actor) = setup()?;
        let repo = NotificationRepository::new(db.pool.clone());
        let post_subject_id = Uuid::new_v4().to_string();

        repo.create(&notification_with_subject_id(
            recipient,
            actor,
            "post",
            post_subject_id.clone(),
        ))?;
        repo.create(&notification_with_subject_id(
            recipient,
            actor,
            "post",
            post_subject_id,
        ))?;
        repo.create(&notification(recipient, actor, "dm_conversation"))?;

        let listed = repo.list_for_user(&recipient, 10, 0)?;
        assert_eq!(listed.len(), 3);
        assert_eq!(repo.unread_count(&recipient)?, 3);

        let grouped = repo.unread_counts_by_subject(&recipient)?;
        assert_eq!(grouped.len(), 2);
        assert!(grouped.iter().any(|count| count.subject_type == "post"));
        assert!(grouped
            .iter()
            .any(|count| count.subject_type == "dm_conversation"));
        Ok(())
    }

    #[test]
    fn test_mark_read() -> Result<()> {
        let (db, recipient, actor) = setup()?;
        let repo = NotificationRepository::new(db.pool.clone());

        let n = notification(recipient, actor, "post");
        repo.create(&n)?;
        repo.create(&notification(recipient, actor, "post"))?;
        assert_eq!(repo.unread_count(&recipient)?, 2);

        repo.mark_read_for_user(&recipient, &n.id)?;
        assert_eq!(repo.unread_count(&recipient)?, 1);

        repo.mark_all_read(&recipient)?;
        assert_eq!(repo.unread_count(&recipient)?, 0);
        Ok(())
    }
}
