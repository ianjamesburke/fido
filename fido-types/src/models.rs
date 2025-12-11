use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{ColorScheme, SortOrder, VoteDirection};

// Custom serde module for DateTime to ensure RFC3339 string format
mod datetime_format {
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
    pub github_id: Option<i64>,
    #[serde(default)]
    pub github_login: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub content: String,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
    pub upvotes: i32,
    pub downvotes: i32,
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

// Request/Response types for API
#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePostRequest {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct WebUserContextResponse {
    pub user: Option<User>,
    pub is_web_mode: bool,
    pub isolation_active: bool,
}
