use axum::{
    extract::{Query, State},
    Json,
};
use fido_types::{MarkNotificationsReadRequest, Notification, NotificationUnreadCount};
use serde::Deserialize;

use crate::{
    api::{ApiError, ApiResult},
    http::AuthenticatedUser,
    services::notifications::NotificationService,
    state::AppState,
};

#[derive(Deserialize)]
pub struct NotificationsQuery {
    #[serde(default = "default_limit")]
    limit: i32,
    #[serde(default)]
    offset: i32,
}

fn default_limit() -> i32 {
    25
}

pub async fn list_notifications(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<NotificationsQuery>,
) -> ApiResult<Json<Vec<Notification>>> {
    if query.limit <= 0 || query.limit > 100 {
        return Err(ApiError::BadRequest(
            "limit must be between 1 and 100".to_string(),
        ));
    }
    if query.offset < 0 {
        return Err(ApiError::BadRequest(
            "offset must be non-negative".to_string(),
        ));
    }

    let service = NotificationService::new(state.repos.clone(), state.event_bus.clone());
    Ok(Json(service.list_for_user(
        &user_id,
        query.limit,
        query.offset,
    )?))
}

pub async fn unread_count(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<NotificationUnreadCount>>> {
    let service = NotificationService::new(state.repos.clone(), state.event_bus.clone());
    Ok(Json(service.unread_counts(&user_id)?))
}

pub async fn mark_read(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<MarkNotificationsReadRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if payload.all && payload.notification_id.is_some() {
        return Err(ApiError::BadRequest(
            "Use either notification_id or all, not both".to_string(),
        ));
    }
    if !payload.all && payload.notification_id.is_none() {
        return Err(ApiError::BadRequest(
            "notification_id is required unless all is true".to_string(),
        ));
    }

    let service = NotificationService::new(state.repos.clone(), state.event_bus.clone());
    service.mark_read(&user_id, payload.notification_id)?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use chrono::Utc;
    use fido_types::{NotificationType, User};
    use uuid::Uuid;

    use crate::{db::repositories::Repositories, db::Database};

    fn setup_state() -> anyhow::Result<(AppState, Uuid, Uuid)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let repos = Repositories::new(db.pool.clone());
        let recipient = User {
            id: Uuid::new_v4(),
            username: "recipient".to_string(),
            bio: None,
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        };
        let actor = User {
            id: Uuid::new_v4(),
            username: "actor".to_string(),
            bio: None,
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        };
        repos.users.create(&recipient)?;
        repos.users.create(&actor)?;

        let state = {
            let _guard = crate::test_utils::env_lock().lock().unwrap();
            std::env::set_var("FIDO_TOKEN_KEY", STANDARD.encode([9u8; 32]));
            AppState::new_with_repos(db, repos)?
        };
        Ok((state, recipient.id, actor.id))
    }

    #[tokio::test]
    async fn lists_counts_and_marks_notifications_read() -> anyhow::Result<()> {
        let (state, recipient_id, actor_id) = setup_state()?;
        let service = NotificationService::new(state.repos.clone(), state.event_bus.clone());
        let notification = service
            .create(
                recipient_id,
                NotificationType::Mention,
                actor_id,
                "community",
                Uuid::new_v4().to_string(),
            )
            .expect("notification creates")
            .expect("not self notification");

        let listed = list_notifications(
            State(state.clone()),
            AuthenticatedUser(recipient_id),
            Query(NotificationsQuery {
                limit: 25,
                offset: 0,
            }),
        )
        .await
        .expect("notifications list succeeds")
        .0;
        assert_eq!(listed.len(), 1);

        let counts = unread_count(State(state.clone()), AuthenticatedUser(recipient_id))
            .await
            .expect("unread count succeeds")
            .0;
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].count, 1);

        let _ = mark_read(
            State(state.clone()),
            AuthenticatedUser(recipient_id),
            Json(MarkNotificationsReadRequest {
                notification_id: Some(notification.id),
                all: false,
            }),
        )
        .await
        .expect("mark read succeeds");
        assert!(unread_count(State(state), AuthenticatedUser(recipient_id))
            .await
            .expect("unread count succeeds")
            .0
            .is_empty());
        Ok(())
    }
}
