use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fido_types::{Channel, Community, Membership, MembershipRole};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    http::AuthenticatedUser,
    services::communities::{BrowseCommunity, CommunityService, CommunityView},
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct BrowseCommunityResponse {
    pub github_repo_id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub community: Option<CommunityResponse>,
    pub membership: Option<MembershipResponse>,
}

#[derive(Debug, Serialize)]
pub struct CommunityResponse {
    pub id: String,
    pub github_repo_id: i64,
    pub owner: String,
    pub name: String,
    pub claimed_by: Option<String>,
    pub require_thread_approval: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub id: String,
    pub community_id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct MembershipResponse {
    pub community_id: String,
    pub user_id: String,
    pub role: MembershipRole,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CommunityViewResponse {
    pub community: CommunityResponse,
    pub membership: Option<MembershipResponse>,
    pub member_count: i64,
    pub channels: Vec<ChannelResponse>,
}

#[derive(Debug, Deserialize)]
pub struct JoinCommunityRequest {
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CommunityMemberResponse {
    pub username: String,
    pub role: MembershipRole,
}

/// GET /communities/browse - Browse the user's starred repos with community state
pub async fn browse_communities(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<BrowseCommunityResponse>>> {
    let service = CommunityService::new(state.repos.clone(), state.github_service.clone());
    let communities = service.browse_starred(user_id).await?;

    Ok(Json(
        communities.into_iter().map(map_browse_community).collect(),
    ))
}

/// POST /communities/join - Join a community, lazily creating it from a GitHub repo
pub async fn join_community(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(request): Json<JoinCommunityRequest>,
) -> ApiResult<Json<CommunityViewResponse>> {
    validate_repo_name(&request.owner, "owner")?;
    validate_repo_name(&request.name, "name")?;

    let service = CommunityService::new(state.repos.clone(), state.github_service.clone());
    let view = service.join(user_id, &request.owner, &request.name).await?;

    Ok(Json(map_community_view(view)))
}

/// GET /communities - List communities the authenticated user has joined
pub async fn list_communities(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<Vec<CommunityViewResponse>>> {
    let service = CommunityService::new(state.repos.clone(), state.github_service.clone());
    let communities = service.list_joined(user_id)?;

    Ok(Json(
        communities.into_iter().map(map_community_view).collect(),
    ))
}

/// GET /communities/:id - Get a community by id
pub async fn get_community(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(community_id): Path<Uuid>,
) -> ApiResult<Json<CommunityViewResponse>> {
    let service = CommunityService::new(state.repos.clone(), state.github_service.clone());
    Ok(Json(map_community_view(
        service.get_view(user_id, community_id)?,
    )))
}

/// POST /communities/:id/claim - Claim admin role after GitHub permission verification
pub async fn claim_community(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(community_id): Path<Uuid>,
) -> ApiResult<Json<CommunityViewResponse>> {
    let service = CommunityService::new(state.repos.clone(), state.github_service.clone());
    Ok(Json(map_community_view(
        service.claim(user_id, community_id).await?,
    )))
}

/// GET /communities/:id/members - List a community's members, admins first
pub async fn list_members(
    State(state): State<AppState>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Path(community_id): Path<Uuid>,
) -> ApiResult<Json<Vec<CommunityMemberResponse>>> {
    let service = CommunityService::new(state.repos.clone(), state.github_service.clone());
    let members = service.list_members_with_usernames(&community_id)?;

    Ok(Json(
        members
            .into_iter()
            .map(|(username, role)| CommunityMemberResponse { username, role })
            .collect(),
    ))
}

/// DELETE /communities/:id/membership - Leave a community
pub async fn leave_community(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(community_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let service = CommunityService::new(state.repos.clone(), state.github_service.clone());
    service.leave(user_id, community_id)?;
    Ok(StatusCode::NO_CONTENT)
}

fn map_browse_community(item: BrowseCommunity) -> BrowseCommunityResponse {
    BrowseCommunityResponse {
        github_repo_id: item.github_repo_id,
        owner: item.owner,
        name: item.name,
        full_name: item.full_name,
        private: item.private,
        community: item.community.map(map_community),
        membership: item.membership.map(map_membership),
    }
}

fn map_community_view(view: CommunityView) -> CommunityViewResponse {
    CommunityViewResponse {
        community: map_community(view.community),
        membership: view.membership.map(map_membership),
        member_count: view.member_count,
        channels: view.channels.into_iter().map(map_channel).collect(),
    }
}

fn map_community(community: Community) -> CommunityResponse {
    CommunityResponse {
        id: community.id.to_string(),
        github_repo_id: community.github_repo_id,
        owner: community.owner,
        name: community.name,
        claimed_by: community.claimed_by.map(|id| id.to_string()),
        require_thread_approval: community.require_thread_approval,
        created_at: community.created_at.to_rfc3339(),
    }
}

fn map_channel(channel: Channel) -> ChannelResponse {
    ChannelResponse {
        id: channel.id.to_string(),
        community_id: channel.community_id.to_string(),
        name: channel.name,
        created_at: channel.created_at.to_rfc3339(),
    }
}

fn map_membership(membership: Membership) -> MembershipResponse {
    MembershipResponse {
        community_id: membership.community_id.to_string(),
        user_id: membership.user_id.to_string(),
        role: membership.role,
        created_at: membership.created_at.to_rfc3339(),
    }
}

fn validate_repo_name(value: &str, field: &str) -> ApiResult<()> {
    if value.trim().is_empty() || value.contains('/') {
        return Err(ApiError::BadRequest(format!("Invalid repo {}", field)));
    }
    Ok(())
}
