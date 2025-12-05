use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    db::repositories::{DirectMessageRepository, UserRepository},
    state::AppState,
};
use fido_types::{DirectMessage, SendMessageRequest};

/// Extract user ID from session token header
fn get_user_from_headers(state: &AppState, headers: &HeaderMap) -> Result<Uuid, ApiError> {
    let token = headers
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing session token".to_string()))?;
    
    state
        .get_authenticated_user_id_from_token(token)
        .ok_or_else(|| ApiError::Unauthorized("Invalid session token".to_string()))
}

/// GET /dms/conversations - List conversations for current user
pub async fn get_conversations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    // Get authenticated user from session token
    let user_id = get_user_from_headers(&state, &headers)?;

    let pool = state.db.pool.clone();
    let dm_repo = DirectMessageRepository::new(pool.clone());
    let user_repo = UserRepository::new(pool);

    // Get list of user IDs with conversations
    let conversation_user_ids = dm_repo
        .get_conversations_list(&user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Build conversation list with user info and unread count
    let mut conversations = Vec::new();
    for other_user_id in conversation_user_ids {
        let user = user_repo
            .get_by_id(&other_user_id)
            .map_err(|e| ApiError::InternalError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        // Get messages for this conversation
        let messages = dm_repo
            .get_conversation(&user_id, &other_user_id)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        let unread_count = messages
            .iter()
            .filter(|m| m.to_user_id == user_id && !m.is_read)
            .count();

        // Skip conversations with no visible messages (all deleted)
        if messages.is_empty() {
            continue;
        }

        // Get last message info
        let (last_message, last_message_time) = if let Some(last_msg) = messages.last() {
            (last_msg.content.clone(), last_msg.created_at.to_rfc3339())
        } else {
            ("No messages yet".to_string(), Utc::now().to_rfc3339())
        };

        conversations.push(serde_json::json!({
            "other_user_id": other_user_id.to_string(),
            "other_username": user.username,
            "last_message": last_message,
            "last_message_time": last_message_time,
            "unread_count": unread_count
        }));
    }

    Ok(Json(conversations))
}

/// GET /dms/conversations/:user_id - Get conversation with specific user
pub async fn get_conversation(
    State(state): State<AppState>,
    Path(other_user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<DirectMessage>>> {
    // Parse other user ID
    let other_user_id = Uuid::parse_str(&other_user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    // Get authenticated user from session token
    let user_id = get_user_from_headers(&state, &headers)?;

    let pool = state.db.pool.clone();
    let dm_repo = DirectMessageRepository::new(pool.clone());
    let user_repo = UserRepository::new(pool);

    // Verify other user exists
    user_repo
        .get_by_id(&other_user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Validate user is participant (can only view own conversations)
    // This is implicitly validated by using the authenticated user_id

    // Get conversation
    let mut messages = dm_repo
        .get_conversation(&user_id, &other_user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Enrich messages with usernames
    for msg in &mut messages {
        let from_user = user_repo
            .get_by_id(&msg.from_user_id)
            .map_err(|e| ApiError::InternalError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound("Sender not found".to_string()))?;
        
        let to_user = user_repo
            .get_by_id(&msg.to_user_id)
            .map_err(|e| ApiError::InternalError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound("Recipient not found".to_string()))?;
        
        msg.from_username = from_user.username;
        msg.to_username = to_user.username;
    }

    // Mark messages as read
    dm_repo
        .mark_as_read(&user_id, &other_user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(messages))
}

/// POST /dms/mark-read/:user_id - Mark messages as read for a specific user
pub async fn mark_messages_read(
    State(state): State<AppState>,
    Path(other_user_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // Parse other user ID
    let other_user_id = Uuid::parse_str(&other_user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    // Get authenticated user from session token
    let user_id = get_user_from_headers(&state, &headers)?;

    let pool = state.db.pool.clone();
    let dm_repo = DirectMessageRepository::new(pool);

    // Mark messages as read
    dm_repo
        .mark_as_read(&user_id, &other_user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Messages marked as read"
    })))
}

/// POST /dms - Send a direct message
pub async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SendMessageRequest>,
) -> ApiResult<Json<DirectMessage>> {
    // Validate content
    if payload.content.is_empty() {
        return Err(ApiError::BadRequest("Message content cannot be empty".to_string()));
    }

    // Get authenticated user from session token
    let from_user_id = get_user_from_headers(&state, &headers)?;

    let pool = state.db.pool.clone();
    let user_repo = UserRepository::new(pool.clone());
    let dm_repo = DirectMessageRepository::new(pool);

    // Get recipient by username
    let to_user = user_repo
        .get_by_username(&payload.to_username)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| {
            ApiError::NotFound(format!("User '{}' not found", payload.to_username))
        })?;

    // Validate sender authentication (already done via from_user_id)
    // Cannot send message to yourself
    if from_user_id == to_user.id {
        return Err(ApiError::BadRequest(
            "Cannot send message to yourself".to_string(),
        ));
    }

    // Get sender username
    let from_user = user_repo
        .get_by_id(&from_user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Sender not found".to_string()))?;

    // Create message
    let message = DirectMessage {
        id: Uuid::new_v4(),
        from_user_id,
        to_user_id: to_user.id,
        from_username: from_user.username,
        to_username: to_user.username,
        content: payload.content,
        created_at: Utc::now(),
        is_read: false,
    };

    // Store message (old deleted messages stay hidden)
    dm_repo
        .create(&message)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(message))
}

/// DELETE /dms/conversations/:user_id - Delete conversation with specific user
pub async fn delete_conversation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(other_user_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // Get authenticated user from session token
    let user_id = get_user_from_headers(&state, &headers)?;

    // Parse other user ID
    let other_user_id = Uuid::parse_str(&other_user_id)
        .map_err(|_| ApiError::BadRequest("Invalid user ID format".to_string()))?;

    let pool = state.db.pool.clone();
    let dm_repo = DirectMessageRepository::new(pool);

    // Delete all messages between these two users
    dm_repo
        .delete_conversation(&user_id, &other_user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Conversation deleted"
    })))
}
