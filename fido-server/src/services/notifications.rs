//! Notification creation and read-state business logic.

use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::ApiResult,
    db::repositories::Repositories,
    events::{ServerEvent, SharedEventBus},
};
use fido_types::{Notification, NotificationType, NotificationUnreadCount};

pub struct NotificationService {
    repos: Repositories,
    event_bus: SharedEventBus,
}

impl NotificationService {
    pub fn new(repos: Repositories, event_bus: SharedEventBus) -> Self {
        Self { repos, event_bus }
    }

    pub fn create(
        &self,
        user_id: Uuid,
        notification_type: NotificationType,
        actor_id: Uuid,
        subject_type: impl Into<String>,
        subject_id: impl Into<String>,
    ) -> ApiResult<Option<Notification>> {
        if user_id == actor_id {
            return Ok(None);
        }

        let notification = Notification {
            id: Uuid::new_v4(),
            user_id,
            notification_type,
            actor_id,
            subject_type: subject_type.into(),
            subject_id: subject_id.into(),
            read: false,
            created_at: Utc::now(),
        };

        self.repos.notifications.create(&notification)?;
        self.event_bus
            .emit(ServerEvent::NotificationCreated(notification.clone()))?;
        Ok(Some(notification))
    }

    pub fn list_for_user(
        &self,
        user_id: &Uuid,
        limit: i32,
        offset: i32,
    ) -> ApiResult<Vec<Notification>> {
        Ok(self
            .repos
            .notifications
            .list_for_user(user_id, limit, offset)?)
    }

    pub fn unread_counts(&self, user_id: &Uuid) -> ApiResult<Vec<NotificationUnreadCount>> {
        Ok(self.repos.notifications.unread_counts_by_subject(user_id)?)
    }

    pub fn mark_read(&self, user_id: &Uuid, notification_id: Option<Uuid>) -> ApiResult<()> {
        if let Some(notification_id) = notification_id {
            self.repos
                .notifications
                .mark_read_for_user(user_id, &notification_id)?;
        } else {
            self.repos.notifications.mark_all_read(user_id)?;
        }
        Ok(())
    }
}
