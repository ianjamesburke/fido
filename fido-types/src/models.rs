use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{
    ActivityKind, ActivityState, ColorScheme, DmConversationState, MembershipRole,
    NotificationType, SortOrder, VoteDirection,
};

// Custom serde module for DateTime to ensure RFC3339 string format
pub(crate) mod datetime_format {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.to_rfc3339();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<DateTime<Utc>>().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub bio: Option<String>,
    #[serde(with = "datetime_format")]
    pub join_date: DateTime<Utc>,
    pub is_test_user: bool,
    #[serde(default)]
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub community_id: Uuid,
    pub content: String,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
    pub upvotes: i32,
    pub downvotes: i32,
    /// Whether the thread is approved (top-level threads may await admin approval)
    pub approved: bool,
    pub hashtags: Vec<String>,
    /// User's vote on this post (if authenticated)
    #[serde(default)]
    pub user_vote: Option<String>,
    /// Parent post ID for replies (None for top-level posts)
    #[serde(default)]
    pub parent_post_id: Option<Uuid>,
    /// Number of replies to this post
    #[serde(default)]
    pub reply_count: i32,
    /// User ID being replied to (for @mentions in replies)
    #[serde(default)]
    pub reply_to_user_id: Option<Uuid>,
    /// Username being replied to (for display purposes)
    #[serde(default)]
    pub reply_to_username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub user_id: Uuid,
    pub post_id: Uuid,
    pub direction: VoteDirection,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    pub id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    #[serde(default)]
    pub from_username: String,
    #[serde(default)]
    pub to_username: String,
    pub content: String,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
    pub is_read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: Uuid,
    pub username: String,
    pub bio: Option<String>,
    pub karma: i32,
    pub post_count: i32,
    #[serde(with = "datetime_format")]
    pub join_date: DateTime<Utc>,
    pub recent_hashtags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfileView {
    pub id: String,
    pub username: String,
    pub bio: Option<String>,
    pub join_date: String,
    pub follower_count: usize,
    pub following_count: usize,
    pub post_count: usize,
    pub relationship: RelationshipStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RelationshipStatus {
    #[serde(rename = "self")]
    Self_,
    MutualFriends,
    Following,
    FollowsYou,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub user_id: Uuid,
    pub color_scheme: ColorScheme,
    pub sort_order: SortOrder,
    pub max_posts_display: i32,
    pub emoji_enabled: bool,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            color_scheme: ColorScheme::default(),
            sort_order: SortOrder::default(),
            max_posts_display: 25,
            emoji_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: Uuid,
    pub github_repo_id: i64,
    pub owner: String,
    pub name: String,
    pub claimed_by: Option<Uuid>,
    pub require_thread_approval: bool,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub community_id: Uuid,
    pub name: String,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Membership {
    pub community_id: Uuid,
    pub user_id: Uuid,
    pub role: MembershipRole,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub actor_id: Uuid,
    pub subject_type: String,
    pub subject_id: String,
    pub read: bool,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationUnreadCount {
    pub subject_type: String,
    pub subject_id: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkNotificationsReadRequest {
    #[serde(default)]
    pub notification_id: Option<Uuid>,
    #[serde(default)]
    pub all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmConversation {
    pub user_a: Uuid,
    pub user_b: Uuid,
    pub state: DmConversationState,
    pub initiator_id: Uuid,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

/// A GitHub issue or PR surfaced in a community feed. Read-only ambient
/// content — never a post: no votes, no replies, no post id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub github_id: i64,
    pub kind: ActivityKind,
    pub number: i64,
    pub title: String,
    pub author_login: String,
    pub state: ActivityState,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
    pub html_url: String,
}

// Request/Response types for API
#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub community_id: Uuid,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateReplyRequest {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePostRequest {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteRequest {
    pub direction: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub to_username: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendChannelMessageRequest {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateBioRequest {
    pub bio: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateConfigRequest {
    pub color_scheme: Option<String>,
    pub sort_order: Option<String>,
    pub max_posts_display: Option<i32>,
    pub emoji_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: User,
    pub session_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

#[cfg(test)]
mod activity_tests {
    use super::*;
    use crate::enums::{ActivityKind, ActivityState};

    #[test]
    fn activity_item_serde_round_trip() {
        let item = ActivityItem {
            github_id: 123456,
            kind: ActivityKind::PullRequest,
            number: 42,
            title: "Add dark mode".to_string(),
            author_login: "alice".to_string(),
            state: ActivityState::Merged,
            created_at: "2026-07-01T12:00:00Z".parse().unwrap(),
            html_url: "https://github.com/o/r/pull/42".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"pull_request\""));
        assert!(json.contains("\"merged\""));
        let back: ActivityItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.github_id, 123456);
        assert_eq!(back.kind, ActivityKind::PullRequest);
        assert_eq!(back.state, ActivityState::Merged);
    }
}
