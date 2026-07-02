use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{Duration, Utc};
use fido_types::{Channel, Message, SendChannelMessageRequest};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    http::{extract_client_ip, extract_user_agent, AuthenticatedUser},
    security::validation::validate_post_content,
    security::{AuditEvent, AuditEventType},
    services::chat::{ChatService, DEFAULT_MESSAGE_LIMIT},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct MessageHistoryQuery {
    #[serde(default)]
    before: Option<Uuid>,
    #[serde(default)]
    after: Option<Uuid>,
    #[serde(default = "default_message_limit")]
    limit: i32,
}

fn default_message_limit() -> i32 {
    DEFAULT_MESSAGE_LIMIT
}

fn check_channel_message_rate_limit(state: &AppState, user_id: &Uuid) -> Result<(), ApiError> {
    let last_message_at = state
        .repos
        .rate_limits
        .get_last_dm_at(user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if let Some(last_message) = last_message_at {
        let elapsed = Utc::now().signed_duration_since(last_message);
        let rate_limit_duration = Duration::seconds(1);
        if elapsed < rate_limit_duration {
            return Err(ApiError::TooManyRequests(
                "Rate limit exceeded. Please wait 1 second before sending another message."
                    .to_string(),
            ));
        }
    }

    Ok(())
}

fn update_channel_message_rate_limit(state: &AppState, user_id: &Uuid) -> Result<(), ApiError> {
    state
        .repos
        .rate_limits
        .update_last_dm_at(user_id, Utc::now())
        .map_err(|e| ApiError::InternalError(e.to_string()))
}

pub async fn list_community_channels(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(community_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Channel>>> {
    let service = ChatService::new(state.repos.clone(), state.event_bus.clone());
    Ok(Json(service.list_channels(user_id, community_id)?))
}

pub async fn get_channel_messages(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<MessageHistoryQuery>,
) -> ApiResult<Json<Vec<Message>>> {
    if query.before.is_some() && query.after.is_some() {
        return Err(ApiError::BadRequest(
            "before and after cursors are mutually exclusive".to_string(),
        ));
    }

    let service = ChatService::new(state.repos.clone(), state.event_bus.clone());
    Ok(Json(service.list_messages(
        user_id,
        channel_id,
        query.before,
        query.after,
        query.limit,
    )?))
}

pub async fn send_channel_message(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    headers: HeaderMap,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<SendChannelMessageRequest>,
) -> ApiResult<Json<Message>> {
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    if let Err(e) = validate_post_content(&payload.content) {
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::ValidationFailure)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details(format!("Channel message validation failed: {}", e)),
        );
        return Err(ApiError::BadRequest(e.to_string()));
    }

    check_channel_message_rate_limit(&state, &user_id)?;
    let service = ChatService::new(state.repos.clone(), state.event_bus.clone());
    let message = service.send_message(user_id, channel_id, payload.content)?;
    update_channel_message_rate_limit(&state, &user_id)?;

    Ok(Json(message))
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use fido_types::{Community, Membership, MembershipRole, User};

    use crate::{db::repositories::Repositories, db::Database};

    fn setup_state() -> anyhow::Result<(AppState, Uuid, Uuid, Uuid)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let repos = Repositories::new(db.pool.clone());
        let user = User {
            id: Uuid::new_v4(),
            username: "member".to_string(),
            bio: None,
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        };
        repos.users.create(&user)?;

        let community = Community {
            id: Uuid::new_v4(),
            github_repo_id: 99,
            owner: "octocat".to_string(),
            name: "hello".to_string(),
            claimed_by: None,
            require_thread_approval: false,
            created_at: Utc::now(),
        };
        repos.communities.create(&community)?;

        let channel = Channel {
            id: Uuid::new_v4(),
            community_id: community.id,
            name: "general".to_string(),
            created_at: Utc::now(),
        };
        repos.channels.create(&channel)?;
        repos.memberships.insert(&Membership {
            community_id: community.id,
            user_id: user.id,
            role: MembershipRole::Member,
            created_at: Utc::now(),
        })?;

        let state = {
            let _guard = crate::test_utils::env_lock().lock().unwrap();
            std::env::set_var("FIDO_TOKEN_KEY", STANDARD.encode([8u8; 32]));
            AppState::new_with_repos(db, repos)?
        };
        Ok((state, user.id, community.id, channel.id))
    }

    #[tokio::test]
    async fn handler_sends_and_lists_channel_messages() -> anyhow::Result<()> {
        let (state, user_id, community_id, channel_id) = setup_state()?;

        let channels = list_community_channels(
            State(state.clone()),
            AuthenticatedUser(user_id),
            Path(community_id),
        )
        .await
        .expect("member can list channels")
        .0;
        assert_eq!(channels.len(), 1);

        let sent = send_channel_message(
            State(state.clone()),
            AuthenticatedUser(user_id),
            HeaderMap::new(),
            Path(channel_id),
            Json(SendChannelMessageRequest {
                content: "hello channel".to_string(),
            }),
        )
        .await
        .expect("member can send message")
        .0;
        assert_eq!(sent.channel_id, channel_id);

        let messages = get_channel_messages(
            State(state),
            AuthenticatedUser(user_id),
            Path(channel_id),
            Query(MessageHistoryQuery {
                before: None,
                after: None,
                limit: 10,
            }),
        )
        .await
        .expect("member can list messages")
        .0;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "hello channel");

        Ok(())
    }

    #[tokio::test]
    async fn rejects_mutually_exclusive_cursors() -> anyhow::Result<()> {
        let (state, user_id, _community_id, channel_id) = setup_state()?;

        let err = get_channel_messages(
            State(state),
            AuthenticatedUser(user_id),
            Path(channel_id),
            Query(MessageHistoryQuery {
                before: Some(Uuid::new_v4()),
                after: Some(Uuid::new_v4()),
                limit: 10,
            }),
        )
        .await
        .expect_err("both cursors should be rejected");

        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }
}
