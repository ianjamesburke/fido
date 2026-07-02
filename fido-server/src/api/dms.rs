use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    http::AuthenticatedUser,
    services::dms::{ConversationSummary, DMService, DmRequestSummary},
    state::AppState,
};
use fido_types::{DirectMessage, SendMessageRequest};

/// Check if user has exceeded DM rate limit (1 DM per 1 second)
fn check_dm_rate_limit(state: &AppState, user_id: &Uuid) -> Result<(), ApiError> {
    let last_dm_at = state
        .repos
        .rate_limits
        .get_last_dm_at(user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if let Some(last_dm) = last_dm_at {
        let now = Utc::now();
        let time_since_last_dm = now.signed_duration_since(last_dm);
        let rate_limit_duration = Duration::seconds(1);

        if time_since_last_dm < rate_limit_duration {
            return Err(ApiError::TooManyRequests(
                "Rate limit exceeded. Please wait 1 second before sending another message."
                    .to_string(),
            ));
        }
    }

    Ok(())
}

/// Update the rate limit timestamp after successful DM creation
fn update_dm_rate_limit(state: &AppState, user_id: &Uuid) -> Result<(), ApiError> {
    state
        .repos
        .rate_limits
        .update_last_dm_at(user_id, Utc::now())
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(())
}

/// GET /dms/conversations - List conversations for current user
pub async fn get_conversations(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<ConversationSummary>>> {
    let service = DMService::new(state.repos.clone(), state.event_bus.clone());
    let conversations = service.get_conversations(&user_id)?;

    Ok(Json(conversations))
}

/// GET /dms/conversations/:user_id - Get conversation with specific user
pub async fn get_conversation(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(other_user_id): Path<String>,
) -> ApiResult<Json<Vec<DirectMessage>>> {
    // Parse other user ID
    let other_user_id = Uuid::parse_str(&other_user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let service = DMService::new(state.repos.clone(), state.event_bus.clone());
    let messages = service.get_conversation(&user_id, &other_user_id)?;

    Ok(Json(messages))
}

/// POST /dms/mark-read/:user_id - Mark messages as read for a specific user
pub async fn mark_messages_read(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(other_user_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // Parse other user ID
    let other_user_id = Uuid::parse_str(&other_user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let service = DMService::new(state.repos.clone(), state.event_bus.clone());
    service.mark_messages_read(&user_id, &other_user_id)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Messages marked as read"
    })))
}

/// GET /dms/requests - List pending DM requests for current user
pub async fn get_pending_requests(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<DmRequestSummary>>> {
    let service = DMService::new(state.repos.clone(), state.event_bus.clone());
    Ok(Json(service.get_pending_requests(&user_id)?))
}

/// POST /dms/requests/:user_id/accept - Accept a pending DM request
pub async fn accept_request(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(requester_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let service = DMService::new(state.repos.clone(), state.event_bus.clone());
    service.accept_request(&user_id, &requester_id)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "DM request accepted"
    })))
}

/// POST /dms/requests/:user_id/decline - Decline a pending DM request
pub async fn decline_request(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(requester_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let service = DMService::new(state.repos.clone(), state.event_bus.clone());
    service.decline_request(&user_id, &requester_id)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "DM request declined"
    })))
}

/// POST /dms - Send a direct message
pub async fn send_message(
    State(state): State<AppState>,
    AuthenticatedUser(from_user_id): AuthenticatedUser,
    Json(payload): Json<SendMessageRequest>,
) -> ApiResult<Json<DirectMessage>> {
    let service = DMService::new(state.repos.clone(), state.event_bus.clone());

    // Check rate limit (1 DM per 1 second)
    check_dm_rate_limit(&state, &from_user_id)?;
    let message = service.send_message(&from_user_id, &payload.to_username, &payload.content)?;

    // Update rate limit timestamp
    update_dm_rate_limit(&state, &from_user_id)?;

    Ok(Json(message))
}

/// DELETE /dms/conversations/:user_id - Delete conversation with specific user
pub async fn delete_conversation(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(other_user_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // Parse other user ID
    let other_user_id = Uuid::parse_str(&other_user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID format".to_string()))?;

    let service = DMService::new(state.repos.clone(), state.event_bus.clone());
    service.delete_conversation(&user_id, &other_user_id)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Conversation deleted"
    })))
}
