use axum::{
    extract::{Path, Query, Request, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    db::{
        isolation::DatabaseAdapter,
        repositories::{HashtagRepository, PostRepository, VoteRepository},
    },
    hashtag::extract_hashtags,
    middleware::RequestUserContext,
    state::AppState,
    test_user_service::TestUserService,
};
use fido_types::{CreatePostRequest, Post, SortOrder, VoteDirection, VoteRequest};

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

/// Extract user context from request extensions (populated by middleware)
fn get_user_context_from_request(request: &Request) -> Option<&RequestUserContext> {
    request.extensions().get::<RequestUserContext>()
}

/// Check if user has exceeded post rate limit (1 post per 10 minutes)
fn check_post_rate_limit(state: &AppState, user_id: &Uuid) -> Result<(), ApiError> {
    let conn = state
        .db
        .pool
        .get()
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Query last post time for this user
    let last_post_at: Option<String> = conn
        .query_row(
            "SELECT last_post_at FROM post_rate_limits WHERE user_id = ?",
            [user_id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if let Some(last_post_str) = last_post_at {
        // Parse the timestamp
        let last_post = DateTime::parse_from_rfc3339(&last_post_str)
            .map_err(|e| ApiError::InternalError(format!("Failed to parse timestamp: {}", e)))?
            .with_timezone(&Utc);

        let now = Utc::now();
        let time_since_last_post = now.signed_duration_since(last_post);
        let rate_limit_duration = Duration::minutes(10);

        if time_since_last_post < rate_limit_duration {
            let remaining = rate_limit_duration - time_since_last_post;
            let minutes = remaining.num_minutes();
            let seconds = remaining.num_seconds() % 60;

            return Err(ApiError::TooManyRequests(format!(
                "Rate limit exceeded. Please wait {}m {}s before posting again.",
                minutes, seconds
            )));
        }
    }

    Ok(())
}

/// Update the rate limit timestamp after successful post creation
fn update_post_rate_limit(state: &AppState, user_id: &Uuid) -> Result<(), ApiError> {
    let conn = state
        .db
        .pool
        .get()
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO post_rate_limits (user_id, last_post_at) VALUES (?, ?)
         ON CONFLICT(user_id) DO UPDATE SET last_post_at = excluded.last_post_at",
        (user_id.to_string(), now),
    )
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(())
}

#[derive(Deserialize)]
pub struct GetPostsQuery {
    #[serde(default = "default_limit")]
    limit: i32,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    hashtag: Option<String>,
    #[serde(default)]
    username: Option<String>,
}

fn default_limit() -> i32 {
    25
}

/// GET /posts - Get posts with sorting and limit (optionally filtered by hashtag)
pub async fn get_posts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<GetPostsQuery>,
) -> ApiResult<Json<Vec<Post>>> {
    let pool = state.db.pool.clone();
    let post_repo = PostRepository::new(pool.clone());
    let hashtag_repo = HashtagRepository::new(pool.clone());
    let vote_repo = VoteRepository::new(pool.clone());

    // Parse sort order
    let sort_order = query
        .sort
        .as_deref()
        .and_then(SortOrder::parse)
        .unwrap_or(SortOrder::Newest);

    // Try to get authenticated user and determine user context
    let user_id = get_user_from_headers(&state, &headers).ok();
    let user_context = if let Some(uid) = user_id {
        // Get user to determine context
        let user_repo = crate::db::repositories::UserRepository::new(pool.clone());
        if let Ok(Some(user)) = user_repo.get_by_id(&uid) {
            if user.is_test_user {
                Some(fido_types::UserContext::test_user(user.username.clone()))
            } else {
                user.github_login.map(fido_types::UserContext::real_user)
            }
        } else {
            None
        }
    } else {
        None
    };

    // Get posts with user context filtering
    let mut posts = if let Some(context) = &user_context {
        // Use test user service for data isolation
        let test_service = TestUserService::new(pool.clone());
        let db_adapter = test_service.get_database_adapter();
        let db_ops = db_adapter.with_context(context);

        // Get posts through isolated database operations
        db_ops
            .get_posts(sort_order, query.limit)
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        // Unauthenticated users see only real user content (no test user posts)
        let mut all_posts = match (&query.hashtag, &query.username) {
            (Some(hashtag), Some(username)) => post_repo
                .get_posts_by_hashtag_and_username(hashtag, username, sort_order, query.limit)
                .map_err(|e| ApiError::InternalError(e.to_string()))?,
            (Some(hashtag), None) => post_repo
                .get_posts_by_hashtag(hashtag, sort_order, query.limit)
                .map_err(|e| ApiError::InternalError(e.to_string()))?,
            (None, Some(username)) => post_repo
                .get_posts_by_username(username, sort_order, query.limit)
                .map_err(|e| ApiError::InternalError(e.to_string()))?,
            (None, None) => post_repo
                .get_posts(sort_order, query.limit)
                .map_err(|e| ApiError::InternalError(e.to_string()))?,
        };

        // Filter out test user posts for unauthenticated users
        let user_repo = crate::db::repositories::UserRepository::new(pool.clone());
        all_posts.retain(|post| {
            if let Ok(Some(user)) = user_repo.get_by_id(&post.author_id) {
                !user.is_test_user
            } else {
                false
            }
        });

        all_posts
    };

    // Track activity if viewing filtered posts and user is authenticated
    if let (Some(ref hashtag), Some(uid)) = (&query.hashtag, user_id) {
        // Update last interaction timestamp for this hashtag
        let _ = hashtag_repo.increment_activity(&uid, hashtag);
    }

    // Populate hashtags and user votes for each post
    for post in &mut posts {
        post.hashtags = hashtag_repo
            .get_by_post(&post.id)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        // If user is authenticated, check their vote on this post
        if let Some(uid) = user_id {
            if let Ok(Some(vote)) = vote_repo.get_vote(&uid, &post.id) {
                post.user_vote = Some(vote.direction.as_str().to_string());
            }
        }
    }

    Ok(Json(posts))
}

/// POST /posts - Create a new post
pub async fn create_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreatePostRequest>,
) -> ApiResult<Json<Post>> {
    // Validate content length
    if payload.content.is_empty() {
        return Err(ApiError::BadRequest(
            "Post content cannot be empty".to_string(),
        ));
    }
    if payload.content.len() > 280 {
        return Err(ApiError::BadRequest(format!(
            "Post content exceeds 280 character limit (current: {})",
            payload.content.len()
        )));
    }

    // Get authenticated user from session token
    let author_id = get_user_from_headers(&state, &headers)?;

    // Check rate limit (1 post per 10 minutes)
    check_post_rate_limit(&state, &author_id)?;

    let pool = state.db.pool.clone();
    let hashtag_repo = HashtagRepository::new(pool.clone());
    let user_repo = crate::db::repositories::UserRepository::new(pool.clone());

    // Get author and determine user context
    let author = user_repo
        .get_by_id(&author_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Author not found".to_string()))?;

    let user_context = if author.is_test_user {
        fido_types::UserContext::test_user(author.username.clone())
    } else {
        let github_login = author
            .github_login
            .ok_or_else(|| ApiError::InternalError("Real user missing GitHub login".to_string()))?;
        fido_types::UserContext::real_user(github_login)
    };

    // Extract hashtags using the new hashtag module
    let hashtags = extract_hashtags(&payload.content);

    // Create post
    let post = Post {
        id: Uuid::new_v4(),
        author_id,
        author_username: author.username,
        content: payload.content,
        created_at: Utc::now(),
        upvotes: 0,
        downvotes: 0,
        hashtags: hashtags.clone(),
        user_vote: None,        // New posts have no votes yet
        parent_post_id: None,   // Top-level post
        reply_count: 0,         // Will be calculated dynamically
        reply_to_user_id: None, // Top-level posts don't reply to anyone
        reply_to_username: None,
    };

    // Store post using user context for proper isolation
    if user_context.is_test_user() {
        // Use isolated database operations for test users
        let test_service = TestUserService::new(pool.clone());
        let db_adapter = test_service.get_database_adapter();
        let db_ops = db_adapter.with_context(&user_context);

        db_ops
            .create_post(&post)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
    } else {
        // Use regular database operations for real users
        let post_repo = PostRepository::new(pool.clone());
        post_repo
            .create(&post)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
    }

    // Update rate limit timestamp
    update_post_rate_limit(&state, &author_id)?;

    // Store hashtags and track activity (only for real users in database)
    if !hashtags.is_empty() && !user_context.is_test_user() {
        hashtag_repo
            .store_hashtags(&post.id, &hashtags)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        // Track user activity for each hashtag
        for hashtag in &hashtags {
            let _ = hashtag_repo.increment_activity(&author_id, hashtag);
        }
    }

    Ok(Json(post))
}

/// POST /posts/:id/vote - Vote on a post
pub async fn vote_on_post(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<VoteRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    // Parse vote direction
    let direction = VoteDirection::parse(&payload.direction).ok_or_else(|| {
        ApiError::BadRequest("Invalid vote direction. Use 'up' or 'down'".to_string())
    })?;

    // Get authenticated user from session token
    let user_id = get_user_from_headers(&state, &headers)?;

    let pool = state.db.pool.clone();
    let vote_repo = VoteRepository::new(pool.clone());
    let post_repo = PostRepository::new(pool.clone());
    let hashtag_repo = HashtagRepository::new(pool);

    // Verify post exists
    post_repo
        .get_by_id(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

    // Upsert vote
    vote_repo
        .upsert_vote(&user_id, &post_id, direction)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Update vote counts
    post_repo
        .update_vote_counts(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Track hashtag activity for this vote
    let hashtags = hashtag_repo
        .get_by_post(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    for hashtag in hashtags {
        let _ = hashtag_repo.increment_activity(&user_id, &hashtag);
    }

    Ok(Json(serde_json::json!({
        "message": "Vote recorded successfully",
        "post_id": post_id,
        "direction": direction.as_str()
    })))
}

/// GET /posts/:id/replies - Get all replies for a post
pub async fn get_replies(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Post>>> {
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    let pool = state.db.pool.clone();
    let post_repo = PostRepository::new(pool.clone());
    let hashtag_repo = HashtagRepository::new(pool.clone());
    let vote_repo = VoteRepository::new(pool);

    // Verify post exists
    post_repo
        .get_by_id(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

    // Get replies
    let mut replies = post_repo
        .get_replies(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Try to get authenticated user (optional)
    let user_id = get_user_from_headers(&state, &headers).ok();

    // Populate hashtags and user votes for each reply
    for reply in &mut replies {
        reply.hashtags = hashtag_repo
            .get_by_post(&reply.id)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        // If user is authenticated, check their vote on this reply
        if let Some(uid) = user_id {
            if let Ok(Some(vote)) = vote_repo.get_vote(&uid, &reply.id) {
                reply.user_vote = Some(vote.direction.as_str().to_string());
            }
        }
    }

    Ok(Json(replies))
}

/// POST /posts/:id/reply - Create a reply to a post
pub async fn create_reply(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<fido_types::CreateReplyRequest>,
) -> ApiResult<Json<Post>> {
    // Parse post ID
    let parent_post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    // Validate content length
    if payload.content.is_empty() {
        return Err(ApiError::BadRequest(
            "Reply content cannot be empty".to_string(),
        ));
    }
    if payload.content.len() > 280 {
        return Err(ApiError::BadRequest(format!(
            "Reply content exceeds 280 character limit (current: {})",
            payload.content.len()
        )));
    }

    // Get authenticated user from session token
    let author_id = get_user_from_headers(&state, &headers)?;

    // Check rate limit for replies (same as posts - 1 per 10 minutes)
    check_post_rate_limit(&state, &author_id)?;

    let pool = state.db.pool.clone();
    let post_repo = PostRepository::new(pool.clone());
    let hashtag_repo = HashtagRepository::new(pool.clone());
    let user_repo = crate::db::repositories::UserRepository::new(pool);

    // Get the post being replied to
    let target_post = post_repo
        .get_by_id(&parent_post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

    // Use the actual parent_post_id for true nested replies
    let actual_parent_id = parent_post_id;

    // Get author username
    let author = user_repo
        .get_by_id(&author_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Author not found".to_string()))?;

    // Extract hashtags using the hashtag module
    let hashtags = extract_hashtags(&payload.content);

    // Determine who is being replied to - always the direct parent's author
    let reply_to_user_id = Some(target_post.author_id);
    let reply_to_username = Some(target_post.author_username.clone());

    // Auto-mention the parent author ONLY if replying to a reply (nested reply)
    // Direct replies to the main post should NOT have mentions
    let final_content = if target_post.parent_post_id.is_some() {
        // This is a nested reply (replying to a reply), add mention
        let mention = format!("@{} ", target_post.author_username);
        if payload.content.starts_with(&mention) {
            payload.content.clone()
        } else {
            format!("{}{}", mention, payload.content)
        }
    } else {
        // This is a direct reply to the main post, no mention needed
        payload.content.clone()
    };

    // Create reply (attached to actual parent for nested replies)
    let reply = Post {
        id: Uuid::new_v4(),
        author_id,
        author_username: author.username,
        content: final_content,
        created_at: Utc::now(),
        upvotes: 0,
        downvotes: 0,
        hashtags: hashtags.clone(),
        user_vote: None,
        parent_post_id: Some(actual_parent_id),
        reply_count: 0, // Will be calculated dynamically
        reply_to_user_id,
        reply_to_username,
    };

    // Store reply
    post_repo
        .create(&reply)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Update rate limit timestamp (replies count toward rate limit)
    update_post_rate_limit(&state, &author_id)?;

    // Store hashtags and track activity
    if !hashtags.is_empty() {
        hashtag_repo
            .store_hashtags(&reply.id, &hashtags)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        // Track user activity for each hashtag
        for hashtag in &hashtags {
            let _ = hashtag_repo.increment_activity(&author_id, hashtag);
        }
    }

    Ok(Json(reply))
}

/// Helper function to verify post ownership
async fn verify_post_ownership(
    state: &AppState,
    headers: &HeaderMap,
    post_id: &Uuid,
) -> Result<(), ApiError> {
    let user_id = get_user_from_headers(state, headers)?;

    let pool = state.db.pool.clone();
    let post_repo = PostRepository::new(pool);

    let post = post_repo
        .get_by_id(post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

    if post.author_id != user_id {
        return Err(ApiError::Forbidden(
            "You don't have permission to modify this post".to_string(),
        ));
    }

    Ok(())
}

/// PUT /posts/:id - Update a post
pub async fn update_post(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<fido_types::UpdatePostRequest>,
) -> ApiResult<Json<Post>> {
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    // Validate content length
    if payload.content.is_empty() {
        return Err(ApiError::BadRequest(
            "Post content cannot be empty".to_string(),
        ));
    }
    if payload.content.len() > 280 {
        return Err(ApiError::BadRequest(format!(
            "Post content exceeds 280 character limit (current: {})",
            payload.content.len()
        )));
    }

    // Verify post ownership
    verify_post_ownership(&state, &headers, &post_id).await?;

    let pool = state.db.pool.clone();
    let post_repo = PostRepository::new(pool.clone());
    let hashtag_repo = HashtagRepository::new(pool);

    // Get existing post
    let mut post = post_repo
        .get_by_id(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

    // Update content
    post.content = payload.content.clone();

    // Extract new hashtags using the new hashtag module
    let new_hashtags = extract_hashtags(&payload.content);
    post.hashtags = new_hashtags.clone();

    // Update post in database
    let conn = state
        .db
        .pool
        .get()
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    conn.execute(
        "UPDATE posts SET content = ? WHERE id = ?",
        (payload.content, post_id.to_string()),
    )
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Update hashtags (delete old ones and insert new ones)
    conn.execute(
        "DELETE FROM hashtags WHERE post_id = ?",
        [post_id.to_string()],
    )
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if !new_hashtags.is_empty() {
        hashtag_repo
            .store_hashtags(&post_id, &new_hashtags)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
    }

    Ok(Json(post))
}

/// DELETE /posts/:id - Delete a post
pub async fn delete_post(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    // Verify post ownership
    verify_post_ownership(&state, &headers, &post_id).await?;

    let pool = state.db.pool.clone();
    let post_repo = PostRepository::new(pool.clone());

    // Check if post exists
    post_repo
        .get_by_id(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

    // Delete post (cascade will handle replies, hashtags, and votes)
    let conn = state
        .db
        .pool
        .get()
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    conn.execute("DELETE FROM posts WHERE id = ?", [post_id.to_string()])
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Post deleted successfully",
        "post_id": post_id
    })))
}

/// GET /posts/:id - Get a single post by ID
pub async fn get_post(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<Post>> {
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    let pool = state.db.pool.clone();
    let post_repo = PostRepository::new(pool.clone());
    let hashtag_repo = HashtagRepository::new(pool.clone());
    let vote_repo = VoteRepository::new(pool);

    // Get post
    let mut post = post_repo
        .get_by_id(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

    // Populate hashtags
    post.hashtags = hashtag_repo
        .get_by_post(&post.id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Try to get authenticated user (optional)
    if let Ok(user_id) = get_user_from_headers(&state, &headers) {
        if let Ok(Some(vote)) = vote_repo.get_vote(&user_id, &post.id) {
            post.user_vote = Some(vote.direction.as_str().to_string());
        }
    }

    Ok(Json(post))
}

/// GET /posts/:id/thread - Get a post with all its nested replies in tree structure
pub async fn get_thread(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    let pool = state.db.pool.clone();
    let post_repo = PostRepository::new(pool.clone());
    let hashtag_repo = HashtagRepository::new(pool.clone());
    let vote_repo = VoteRepository::new(pool);

    // Get the root post
    let mut root_post = post_repo
        .get_by_id(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Post not found".to_string()))?;

    // Populate hashtags for root post
    root_post.hashtags = hashtag_repo
        .get_by_post(&root_post.id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Try to get authenticated user (optional)
    let user_id = get_user_from_headers(&state, &headers).ok();

    if let Some(uid) = user_id {
        if let Ok(Some(vote)) = vote_repo.get_vote(&uid, &root_post.id) {
            root_post.user_vote = Some(vote.direction.as_str().to_string());
        }
    }

    // Get all replies recursively
    let mut replies = post_repo
        .get_replies(&post_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Populate hashtags and user votes for each reply
    for reply in &mut replies {
        reply.hashtags = hashtag_repo
            .get_by_post(&reply.id)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        if let Some(uid) = user_id {
            if let Ok(Some(vote)) = vote_repo.get_vote(&uid, &reply.id) {
                reply.user_vote = Some(vote.direction.as_str().to_string());
            }
        }
    }

    // Return root post with all replies
    Ok(Json(serde_json::json!({
        "post": root_post,
        "replies": replies
    })))
}
