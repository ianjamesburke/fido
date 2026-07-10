use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    http::{extract_client_ip, extract_user_agent, AuthenticatedUser},
    security::validation::validate_post_content,
    security::{AuditEvent, AuditEventType},
    services::posts::PostService,
    state::AppState,
};
use fido_types::{CreatePostRequest, Post, SortOrder, VoteDirection, VoteRequest};

/// Check if user has exceeded post rate limit (1 post per 10 seconds)
fn check_post_rate_limit(state: &AppState, user_id: &Uuid) -> Result<(), ApiError> {
    let last_post_at = state
        .repos
        .rate_limits
        .get_last_post_at(user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if let Some(last_post) = last_post_at {
        let now = Utc::now();
        let time_since_last_post = now.signed_duration_since(last_post);
        // Rate limit: 10 seconds between posts
        let rate_limit_duration = Duration::seconds(10);

        if time_since_last_post < rate_limit_duration {
            let remaining = rate_limit_duration - time_since_last_post;
            let seconds = remaining.num_seconds();

            return Err(ApiError::TooManyRequests(format!(
                "Rate limit exceeded. Please wait {}s before posting again.",
                seconds
            )));
        }
    }

    Ok(())
}

/// Update the rate limit timestamp after successful post creation
fn update_post_rate_limit(state: &AppState, user_id: &Uuid) -> Result<(), ApiError> {
    state
        .repos
        .rate_limits
        .update_last_post_at(user_id, Utc::now())
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(())
}

#[derive(Deserialize)]
pub struct GetPostsQuery {
    community_id: Uuid,
    #[serde(default = "default_limit")]
    limit: i32,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    username: Option<String>,
}

#[derive(Deserialize)]
pub struct PendingPostsQuery {
    #[serde(default = "default_limit")]
    limit: i32,
}

fn default_limit() -> i32 {
    25
}

fn post_service(state: &AppState) -> PostService {
    PostService::new(state.repos.clone(), state.event_bus.clone()).with_activity(
        crate::services::activity::ActivityService::new(
            state.repos.clone(),
            state.github_service.clone(),
        ),
    )
}

/// GET /posts - Get posts with sorting and limit (optionally filtered by username)
pub async fn get_posts(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<GetPostsQuery>,
) -> ApiResult<Json<Vec<Post>>> {
    let service = post_service(&state);

    // Parse and validate sort order - reject invalid values
    let sort_order = if let Some(sort_str) = query.sort.as_deref() {
        SortOrder::parse(sort_str).ok_or_else(|| {
            ApiError::BadRequest(format!(
                "Invalid sort order '{}'. Valid values: Newest, Popular, Controversial",
                sort_str
            ))
        })?
    } else {
        SortOrder::Newest
    };

    // Clamp limit to a safe range so unbounded/negative values can't reach the
    // SQL LIMIT (LIMIT -1 means "no limit" in SQLite).
    let limit = query.limit.clamp(1, 100);

    let posts = service
        .get_posts(
            query.community_id,
            sort_order,
            limit,
            query.username.as_deref(),
            user_id,
        )
        .await?;

    Ok(Json(posts))
}

/// GET /communities/:id/posts/pending - Get pending threads awaiting approval.
pub async fn get_pending_posts(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(community_id): Path<Uuid>,
    Query(query): Query<PendingPostsQuery>,
) -> ApiResult<Json<Vec<Post>>> {
    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    Ok(Json(service.get_pending_posts(
        community_id,
        user_id,
        query.limit,
    )?))
}

/// POST /posts - Create a new post
pub async fn create_post(
    State(state): State<AppState>,
    AuthenticatedUser(author_id): AuthenticatedUser,
    headers: HeaderMap,
    Json(payload): Json<CreatePostRequest>,
) -> ApiResult<Json<Post>> {
    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Validate post content using security validation module
    if let Err(e) = validate_post_content(&payload.content) {
        // Log validation failure
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::ValidationFailure)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details(format!("Post content validation failed: {}", e)),
        );
        return Err(ApiError::BadRequest(e.to_string()));
    }

    // Check rate limit (1 post per 10 minutes)
    check_post_rate_limit(&state, &author_id)?;

    // Get author username
    let author = service.get_user_by_id(&author_id)?;

    // Create post
    let community = state
        .repos
        .communities
        .get_by_id(&payload.community_id)?
        .ok_or_else(|| ApiError::NotFound("Community not found".to_string()))?;

    let post = Post {
        id: Uuid::new_v4(),
        author_id,
        author_username: author.username,
        community_id: payload.community_id,
        content: payload.content,
        created_at: Utc::now(),
        upvotes: 0,
        downvotes: 0,
        approved: !community.require_thread_approval,
        hashtags: Vec::new(),
        user_vote: None,        // New posts have no votes yet
        parent_post_id: None,   // Top-level post
        reply_count: 0,         // Will be calculated dynamically
        reply_to_user_id: None, // Top-level posts don't reply to anyone
        reply_to_username: None,
        github_id: None,
        github_kind: None,
        github_state: None,
        github_html_url: None,
    };

    // Store post
    service.create_post(&post)?;

    // Update rate limit timestamp
    update_post_rate_limit(&state, &author_id)?;

    Ok(Json(post))
}

/// POST /posts/:id/vote - Vote on a post
pub async fn vote_on_post(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(post_id): Path<String>,
    Json(payload): Json<VoteRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    // Parse vote direction
    let direction = VoteDirection::parse(&payload.direction).ok_or_else(|| {
        ApiError::BadRequest("Invalid vote direction. Use 'up' or 'down'".to_string())
    })?;

    service.record_vote(&user_id, &post_id, direction)?;

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
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<Post>>> {
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    let replies = service.get_replies(&post_id, user_id)?;

    Ok(Json(replies))
}

/// POST /posts/:id/reply - Create a reply to a post
pub async fn create_reply(
    State(state): State<AppState>,
    AuthenticatedUser(author_id): AuthenticatedUser,
    Path(post_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<fido_types::CreateReplyRequest>,
) -> ApiResult<Json<Post>> {
    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Parse post ID
    let parent_post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    // Validate reply content using security validation module
    if let Err(e) = validate_post_content(&payload.content) {
        // Log validation failure
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::ValidationFailure)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details(format!("Reply content validation failed: {}", e)),
        );
        return Err(ApiError::BadRequest(e.to_string()));
    }

    // Check rate limit for replies (same as posts - 1 per 10 minutes)
    check_post_rate_limit(&state, &author_id)?;

    // Get the post being replied to
    let target_post = service.get_post(&parent_post_id, author_id)?;

    // Use the actual parent_post_id for true nested replies
    let actual_parent_id = parent_post_id;

    // Get author username
    let author = service.get_user_by_id(&author_id)?;

    // Determine who is being replied to - always the direct parent's author
    let reply_to_user_id = Some(target_post.author_id);
    let reply_to_username = Some(target_post.author_username.clone());

    // Auto-mention the parent author ONLY if replying to a reply (nested reply)
    // Direct replies to the main post should NOT have mentions
    let final_content = if target_post.parent_post_id.is_some() {
        // This is a nested reply (replying to a reply), add mention
        let mention = format!("@{} ", target_post.author_username);
        if payload.content.starts_with(&mention) {
            payload.content
        } else {
            format!("{}{}", mention, payload.content)
        }
    } else {
        // This is a direct reply to the main post, no mention needed
        payload.content
    };

    // Create reply (attached to actual parent for nested replies)
    let reply = Post {
        id: Uuid::new_v4(),
        author_id,
        author_username: author.username,
        community_id: target_post.community_id,
        content: final_content,
        created_at: Utc::now(),
        upvotes: 0,
        downvotes: 0,
        approved: true,
        hashtags: Vec::new(),
        user_vote: None,
        parent_post_id: Some(actual_parent_id),
        reply_count: 0, // Will be calculated dynamically
        reply_to_user_id,
        reply_to_username,
        github_id: None,
        github_kind: None,
        github_state: None,
        github_html_url: None,
    };

    // Store reply
    service.create_post(&reply)?;

    // Update rate limit timestamp (replies count toward rate limit)
    update_post_rate_limit(&state, &author_id)?;

    Ok(Json(reply))
}

/// PUT /posts/:id - Update a post
pub async fn update_post(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(post_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<fido_types::UpdatePostRequest>,
) -> ApiResult<Json<Post>> {
    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    let client_ip = extract_client_ip(&headers);
    let user_agent = extract_user_agent(&headers);

    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    // Validate post content using security validation module
    if let Err(e) = validate_post_content(&payload.content) {
        // Log validation failure
        let _ = state.audit_logger.log(
            AuditEvent::new(AuditEventType::ValidationFailure)
                .with_optional_ip_address(client_ip)
                .with_optional_user_agent(user_agent)
                .with_details(format!("Post update content validation failed: {}", e)),
        );
        return Err(ApiError::BadRequest(e.to_string()));
    }

    // Verify post ownership
    service.verify_ownership(&user_id, &post_id)?;

    // Get existing post
    let mut post = service.get_post(&post_id, user_id)?;

    // Update content
    post.content = payload.content.clone();

    // Update post in database
    service.update_post_content(&post_id, &payload.content)?;

    Ok(Json(post))
}

/// DELETE /posts/:id - Delete a post
pub async fn delete_post(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    // Verify post ownership
    service.verify_ownership(&user_id, &post_id)?;

    // Check if post exists
    let _ = service.get_post(&post_id, user_id)?;

    // Delete post (cascade will handle replies and votes)
    service.delete_post(&post_id)?;

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
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Post>> {
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    let post = service.get_post(&post_id, user_id)?;

    Ok(Json(post))
}

/// GET /posts/:id/thread - Get a post with all its nested replies in tree structure
pub async fn get_thread(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<serde_json::Value>> {
    // Parse post ID
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    let (root_post, replies) = service.get_thread_parts(&post_id, user_id)?;

    // Return root post with all replies
    Ok(Json(serde_json::json!({
        "post": root_post,
        "replies": replies
    })))
}

/// POST /posts/:id/approve - Approve a pending top-level thread.
pub async fn approve_post(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<Post>> {
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    Ok(Json(service.approve_post(user_id, &post_id)?))
}

/// POST /posts/:id/reject - Reject a pending top-level thread.
pub async fn reject_post(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<Post>> {
    let post_id = Uuid::parse_str(&post_id)
        .map_err(|_| ApiError::BadRequest("Invalid post ID".to_string()))?;

    let service = PostService::new(state.repos.clone(), state.event_bus.clone());
    Ok(Json(service.reject_post(user_id, &post_id)?))
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use fido_types::{Community, Membership, MembershipRole, User};

    use crate::{db::repositories::Repositories, db::Database};

    fn test_user(username: &str) -> User {
        User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            bio: None,
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        }
    }

    fn setup_state() -> anyhow::Result<(AppState, Uuid, Uuid, Uuid)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let repos = Repositories::new(db.pool.clone());

        let admin = test_user("admin");
        let member = test_user("member");
        repos.users.create(&admin)?;
        repos.users.create(&member)?;

        let community = Community {
            id: Uuid::new_v4(),
            github_repo_id: 7007,
            owner: "octocat".to_string(),
            name: "threads".to_string(),
            claimed_by: Some(admin.id),
            require_thread_approval: true,
            created_at: Utc::now(),
        };
        repos.communities.create(&community)?;
        repos.memberships.insert(&Membership {
            community_id: community.id,
            user_id: admin.id,
            role: MembershipRole::Admin,
            created_at: Utc::now(),
        })?;
        repos.memberships.insert(&Membership {
            community_id: community.id,
            user_id: member.id,
            role: MembershipRole::Member,
            created_at: Utc::now(),
        })?;

        let state = {
            let _guard = crate::test_utils::env_lock().lock().unwrap();
            std::env::set_var("FIDO_TOKEN_KEY", STANDARD.encode([7u8; 32]));
            AppState::new_with_repos(db, repos)?
        };

        Ok((state, admin.id, member.id, community.id))
    }

    #[tokio::test]
    async fn approval_required_thread_is_pending_until_admin_approves() -> anyhow::Result<()> {
        let (state, admin_id, member_id, community_id) = setup_state()?;

        let created = create_post(
            State(state.clone()),
            AuthenticatedUser(member_id),
            HeaderMap::new(),
            Json(CreatePostRequest {
                community_id,
                content: "needs review".to_string(),
            }),
        )
        .await
        .expect("member can create pending thread")
        .0;
        assert!(!created.approved);

        let visible = get_posts(
            State(state.clone()),
            AuthenticatedUser(member_id),
            Query(GetPostsQuery {
                community_id,
                limit: 10,
                sort: None,
                username: None,
            }),
        )
        .await
        .expect("member can list approved feed")
        .0;
        assert!(visible.is_empty());

        let pending = get_pending_posts(
            State(state.clone()),
            AuthenticatedUser(admin_id),
            Path(community_id),
            Query(PendingPostsQuery { limit: 10 }),
        )
        .await
        .expect("admin can list pending threads")
        .0;
        assert_eq!(pending[0].id, created.id);

        let approved = approve_post(
            State(state.clone()),
            AuthenticatedUser(admin_id),
            Path(created.id.to_string()),
        )
        .await
        .expect("admin can approve")
        .0;
        assert!(approved.approved);

        let visible = get_posts(
            State(state),
            AuthenticatedUser(member_id),
            Query(GetPostsQuery {
                community_id,
                limit: 10,
                sort: None,
                username: None,
            }),
        )
        .await
        .expect("approved thread appears in feed")
        .0;
        assert_eq!(visible[0].id, created.id);

        Ok(())
    }

    #[tokio::test]
    async fn non_admin_cannot_list_or_approve_pending_threads() -> anyhow::Result<()> {
        let (state, _admin_id, member_id, community_id) = setup_state()?;

        let created = create_post(
            State(state.clone()),
            AuthenticatedUser(member_id),
            HeaderMap::new(),
            Json(CreatePostRequest {
                community_id,
                content: "member pending".to_string(),
            }),
        )
        .await
        .expect("member can create pending thread")
        .0;

        let pending_err = get_pending_posts(
            State(state.clone()),
            AuthenticatedUser(member_id),
            Path(community_id),
            Query(PendingPostsQuery { limit: 10 }),
        )
        .await
        .expect_err("member cannot list pending threads");
        assert_eq!(pending_err.into_response().status(), StatusCode::FORBIDDEN);

        let approve_err = approve_post(
            State(state),
            AuthenticatedUser(member_id),
            Path(created.id.to_string()),
        )
        .await
        .expect_err("member cannot approve pending thread");
        assert_eq!(approve_err.into_response().status(), StatusCode::FORBIDDEN);

        Ok(())
    }

    #[tokio::test]
    async fn approved_feed_is_scoped_to_requested_community() -> anyhow::Result<()> {
        let (state, _admin_id, member_id, community_id) = setup_state()?;
        let other_community = Community {
            id: Uuid::new_v4(),
            github_repo_id: 7008,
            owner: "octocat".to_string(),
            name: "other".to_string(),
            claimed_by: None,
            require_thread_approval: false,
            created_at: Utc::now(),
        };
        state.repos.communities.create(&other_community)?;
        state.repos.memberships.insert(&Membership {
            community_id: other_community.id,
            user_id: member_id,
            role: MembershipRole::Member,
            created_at: Utc::now(),
        })?;

        let requested_post = Post {
            id: Uuid::new_v4(),
            author_id: member_id,
            author_username: "member".to_string(),
            community_id,
            content: "requested board".to_string(),
            created_at: Utc::now(),
            upvotes: 0,
            downvotes: 0,
            approved: true,
            hashtags: Vec::new(),
            user_vote: None,
            parent_post_id: None,
            reply_count: 0,
            reply_to_user_id: None,
            reply_to_username: None,
            github_id: None,
            github_kind: None,
            github_state: None,
            github_html_url: None,
        };
        let other_post = Post {
            id: Uuid::new_v4(),
            community_id: other_community.id,
            content: "other board".to_string(),
            ..requested_post.clone()
        };
        state.repos.posts.create(&requested_post)?;
        state.repos.posts.create(&other_post)?;

        let visible = get_posts(
            State(state),
            AuthenticatedUser(member_id),
            Query(GetPostsQuery {
                community_id,
                limit: 10,
                sort: None,
                username: None,
            }),
        )
        .await
        .expect("member can list requested community")
        .0;

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, requested_post.id);
        Ok(())
    }
}
