use reqwest::Client;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::{ApiError, ApiResult};
use fido_types::*;

/// Vote direction for posts
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VoteDirection {
    Up,
    Down,
}

impl std::fmt::Display for VoteDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            VoteDirection::Up => "up",
            VoteDirection::Down => "down",
        };
        write!(f, "{}", s)
    }
}

impl VoteDirection {
    /// Convert from string to VoteDirection
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "up" => Some(VoteDirection::Up),
            "down" => Some(VoteDirection::Down),
            _ => None,
        }
    }
}

/// Summary of a DM conversation as returned by `GET /dms/conversations`
#[derive(Debug, serde::Deserialize)]
pub struct ConversationInfo {
    pub other_user_id: String,
    pub other_username: String,
    pub last_message: String,
    pub unread_count: usize,
    pub state: fido_types::DmConversationState,
    pub initiated_by_me: bool,
}

/// A pending incoming DM request as returned by `GET /dms/requests`
#[derive(Debug, serde::Deserialize)]
pub struct DmRequestInfo {
    pub from_user_id: String,
    pub from_username: String,
}

/// A community member as returned by `GET /communities/:id/members`
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CommunityMemberInfo {
    pub username: String,
    pub role: fido_types::MembershipRole,
}

/// API client for communicating with the Fido server
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    session_token: Option<String>,
}

const DEFAULT_PUBLIC_SERVER_URL: &str = "https://fido-web-production.up.railway.app";

fn read_server_url_from_config() -> Option<String> {
    let config_path = dirs::home_dir()?.join(".fido").join("server_url");
    let raw = std::fs::read_to_string(config_path).ok()?;
    let value = raw.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: impl Into<String>) -> Self {
        let builder = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10));

        let builder = if cfg!(test) {
            // Avoid system proxy lookups in tests (can panic in sandboxed environments).
            builder.no_proxy()
        } else {
            builder
        };

        let client = builder.build().expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.into(),
            session_token: None,
        }
    }

    /// Helper to build API URLs
    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Helper to build URLs with query parameters
    fn build_url_with_params(&self, path: &str, params: &[(&str, &str)]) -> String {
        let mut url = self.build_url(path);
        if !params.is_empty() {
            url.push('?');
            let query_string = params
                .iter()
                .map(|(key, value)| format!("{}={}", key, urlencoding::encode(value)))
                .collect::<Vec<_>>()
                .join("&");
            url.push_str(&query_string);
        }
        url
    }

    /// Set the session token for authenticated requests
    pub fn set_session_token(&mut self, token: Option<String>) {
        self.session_token = token;
    }

    /// Current session token used for authenticated requests.
    pub fn session_token(&self) -> Option<&str> {
        self.session_token.as_deref()
    }

    /// Server URL used to scope local session storage.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// WebSocket URL for the realtime gateway that matches the configured API origin.
    pub fn websocket_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        let websocket_base = if let Some(rest) = base.strip_prefix("https://") {
            format!("wss://{}", rest)
        } else if let Some(rest) = base.strip_prefix("http://") {
            format!("ws://{}", rest)
        } else {
            format!("ws://{}", base)
        };
        format!("{}/ws", websocket_base)
    }

    /// Helper to add session token to request if available
    fn add_auth_header(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(token) = &self.session_token {
            req.header("X-Session-Token", token)
        } else {
            req
        }
    }

    /// Helper to handle API responses
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> ApiResult<T> {
        let status = response.status();

        if status.is_success() {
            response.json().await.map_err(ApiError::from)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let clean_error = Self::clean_error_message(status.as_u16(), &error_text);

            let api_error = match status.as_u16() {
                404 => ApiError::NotFound(clean_error),
                401 => ApiError::Unauthorized(clean_error),
                400 => ApiError::BadRequest(clean_error),
                403 => ApiError::Unauthorized(clean_error), // Add forbidden handling
                429 => ApiError::TooManyRequests(clean_error),
                500..=599 => ApiError::Api(format!("Server error ({})", status.as_u16())),
                _ => ApiError::Api(clean_error),
            };

            Err(api_error)
        }
    }

    fn clean_error_message(status_code: u16, error_text: &str) -> String {
        if error_text.contains("<html>") || error_text.contains("<!DOCTYPE") {
            return format!(
                "Server returned {} error. Please check the server URL.",
                status_code
            );
        }

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(error_text) {
            let error = value
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Request failed");
            let details = value.get("details").and_then(|v| v.as_str());

            if let Some(details) = details {
                if !details.trim().is_empty() {
                    return format!("{}: {}", error, details);
                }
            }
            return error.to_string();
        }

        error_text.to_string()
    }

    // Authentication endpoints

    /// Get list of test users
    pub async fn get_test_users(&self) -> ApiResult<Vec<User>> {
        let url = format!("{}/users/test", self.base_url);
        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    /// Login with username
    pub async fn login(&mut self, username: String) -> ApiResult<LoginResponse> {
        let url = format!("{}/auth/login", self.base_url);
        let request = LoginRequest { username };
        let response = self.client.post(&url).json(&request).send().await?;
        let login_response: LoginResponse = self.handle_response(response).await?;

        // Store session token
        self.session_token = Some(login_response.session_token.clone());

        Ok(login_response)
    }

    // Post endpoints

    /// Get posts with optional limit, sort order, and filters
    pub async fn get_posts(
        &self,
        community_id: Uuid,
        limit: Option<i32>,
        sort: Option<String>,
        hashtag: Option<String>,
        username: Option<String>,
    ) -> ApiResult<Vec<Post>> {
        let mut params = Vec::new();
        params.push(("community_id", community_id.to_string()));

        if let Some(l) = limit {
            params.push(("limit", l.to_string()));
        }
        if let Some(s) = sort {
            params.push(("sort", s));
        }
        if let Some(h) = hashtag {
            params.push(("hashtag", h));
        }
        if let Some(u) = username {
            params.push(("username", u));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let url = self.build_url_with_params("/posts", &params_ref);

        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// List communities joined by the authenticated user
    pub async fn list_communities(&self) -> ApiResult<Vec<CommunityViewResponse>> {
        let url = self.build_url("/communities");
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Browse the authenticated user's starred repos with community state.
    pub async fn browse_communities(&self) -> ApiResult<Vec<BrowseCommunityResponse>> {
        let url = self.build_url("/communities/browse");
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Join a repo community; the server resolves the repo via GitHub and
    /// lazily creates the community on first join.
    pub async fn join_community(
        &self,
        owner: &str,
        name: &str,
    ) -> ApiResult<CommunityViewResponse> {
        let url = self.build_url("/communities/join");
        let request = JoinCommunityRequest {
            owner: owner.to_string(),
            name: name.to_string(),
        };
        let req = self.add_auth_header(self.client.post(&url).json(&request));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Get a community view by id (includes the caller's membership)
    pub async fn get_community(&self, community_id: Uuid) -> ApiResult<CommunityViewResponse> {
        let url = self.build_url(&format!("/communities/{}", community_id));
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// List a community's members, admins first
    pub async fn get_community_members(
        &self,
        community_id: Uuid,
    ) -> ApiResult<Vec<CommunityMemberInfo>> {
        let url = self.build_url(&format!("/communities/{}/members", community_id));
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Claim admin of a community (server verifies GitHub admin permission)
    pub async fn claim_community(&self, community_id: Uuid) -> ApiResult<CommunityViewResponse> {
        let url = self.build_url(&format!("/communities/{}/claim", community_id));
        let req = self.add_auth_header(self.client.post(&url).json(&serde_json::json!({})));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Get pending top-level threads awaiting community admin approval.
    pub async fn get_pending_posts(
        &self,
        community_id: Uuid,
        limit: Option<i32>,
    ) -> ApiResult<Vec<Post>> {
        let mut params = Vec::new();
        if let Some(limit) = limit {
            params.push(("limit", limit.to_string()));
        }
        let params_ref: Vec<(&str, &str)> = params
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        let url = self.build_url_with_params(
            &format!("/communities/{}/posts/pending", community_id),
            &params_ref,
        );
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// List chat channels for a community.
    pub async fn list_community_channels(&self, community_id: Uuid) -> ApiResult<Vec<Channel>> {
        let url = self.build_url(&format!("/communities/{}/channels", community_id));
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Load channel message history. Results are oldest-to-newest.
    pub async fn get_channel_messages(
        &self,
        channel_id: Uuid,
        before: Option<Uuid>,
        after: Option<Uuid>,
        limit: i32,
    ) -> ApiResult<Vec<Message>> {
        let mut params = vec![("limit", limit.to_string())];
        let before_string;
        if let Some(before) = before {
            before_string = before.to_string();
            params.push(("before", before_string));
        }
        let after_string;
        if let Some(after) = after {
            after_string = after.to_string();
            params.push(("after", after_string));
        }
        let params_ref: Vec<(&str, &str)> = params
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        let url =
            self.build_url_with_params(&format!("/channels/{}/messages", channel_id), &params_ref);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Send a message to a chat channel.
    pub async fn send_channel_message(
        &self,
        channel_id: Uuid,
        content: String,
    ) -> ApiResult<Message> {
        let url = self.build_url(&format!("/channels/{}/messages", channel_id));
        let request = SendChannelMessageRequest { content };
        let req = self.add_auth_header(self.client.post(&url).json(&request));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Create a new post
    pub async fn create_post(&self, community_id: Uuid, content: String) -> ApiResult<Post> {
        let url = format!("{}/posts", self.base_url);
        let request = CreatePostRequest {
            community_id,
            content,
        };
        let req = self.add_auth_header(self.client.post(&url).json(&request));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Approve a pending top-level thread.
    pub async fn approve_post(&self, post_id: Uuid) -> ApiResult<Post> {
        let url = self.build_url(&format!("/posts/{}/approve", post_id));
        let req = self.add_auth_header(self.client.post(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Reject a pending top-level thread.
    pub async fn reject_post(&self, post_id: Uuid) -> ApiResult<Post> {
        let url = self.build_url(&format!("/posts/{}/reject", post_id));
        let req = self.add_auth_header(self.client.post(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Vote on a post
    pub async fn vote_on_post(
        &self,
        post_id: Uuid,
        direction: VoteDirection,
    ) -> ApiResult<serde_json::Value> {
        let url = self.build_url(&format!("/posts/{}/vote", post_id));
        let request = VoteRequest {
            direction: direction.to_string(),
        };
        let req = self.add_auth_header(self.client.post(&url).json(&request));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Get a single post by ID
    pub async fn get_post_by_id(&self, post_id: Uuid) -> ApiResult<Post> {
        let url = format!("{}/posts/{}", self.base_url, post_id);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Get replies for a post
    pub async fn get_replies(&self, post_id: Uuid) -> ApiResult<Vec<Post>> {
        let url = format!("{}/posts/{}/replies", self.base_url, post_id);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Create a reply to a post
    pub async fn create_reply(&self, post_id: Uuid, content: String) -> ApiResult<Post> {
        let url = format!("{}/posts/{}/reply", self.base_url, post_id);
        let request = CreateReplyRequest { content };
        let req = self.add_auth_header(self.client.post(&url).json(&request));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Delete a post
    pub async fn delete_post(&self, post_id: Uuid) -> ApiResult<serde_json::Value> {
        let url = format!("{}/posts/{}", self.base_url, post_id);
        let req = self.add_auth_header(self.client.delete(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    // Profile endpoints

    /// Get user profile (own profile - legacy)
    pub async fn get_profile(&self, user_id: Uuid) -> ApiResult<UserProfile> {
        let url = format!("{}/users/{}/profile", self.base_url, user_id);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Update user bio
    pub async fn update_bio(&self, user_id: Uuid, bio: String) -> ApiResult<serde_json::Value> {
        let url = format!("{}/users/{}/profile", self.base_url, user_id);
        let request = UpdateBioRequest { bio };
        let req = self.add_auth_header(self.client.put(&url).json(&request));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    // Direct message endpoints

    /// Get conversations list
    pub async fn get_conversations(&self) -> ApiResult<Vec<ConversationInfo>> {
        let url = format!("{}/dms/conversations", self.base_url);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// List pending DM requests addressed to me
    pub async fn get_pending_dm_requests(&self) -> ApiResult<Vec<DmRequestInfo>> {
        let url = format!("{}/dms/requests", self.base_url);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Accept a pending DM request from a user
    pub async fn accept_dm_request(&self, from_user_id: Uuid) -> ApiResult<()> {
        let url = format!("{}/dms/requests/{}/accept", self.base_url, from_user_id);
        let req = self.add_auth_header(self.client.post(&url));
        let response = req.send().await?;
        let _: serde_json::Value = self.handle_response(response).await?;
        Ok(())
    }

    /// Decline a pending DM request from a user
    pub async fn decline_dm_request(&self, from_user_id: Uuid) -> ApiResult<()> {
        let url = format!("{}/dms/requests/{}/decline", self.base_url, from_user_id);
        let req = self.add_auth_header(self.client.post(&url));
        let response = req.send().await?;
        let _: serde_json::Value = self.handle_response(response).await?;
        Ok(())
    }

    /// Get conversation with specific user
    pub async fn get_conversation(&self, user_id: Uuid) -> ApiResult<Vec<DirectMessage>> {
        let url = format!("{}/dms/conversations/{}", self.base_url, user_id);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Send a direct message
    pub async fn send_message(
        &self,
        to_username: String,
        content: String,
    ) -> ApiResult<DirectMessage> {
        let url = format!("{}/dms", self.base_url);
        let request_body = SendMessageRequest {
            to_username,
            content,
        };
        let req = self.add_auth_header(self.client.post(&url).json(&request_body));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    // Configuration endpoints

    /// Get user configuration
    pub async fn get_config(&self) -> ApiResult<UserConfig> {
        let url = format!("{}/config", self.base_url);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Update user configuration
    pub async fn update_config(&self, request: UpdateConfigRequest) -> ApiResult<UserConfig> {
        let url = format!("{}/config", self.base_url);
        let req = self.add_auth_header(self.client.put(&url).json(&request));
        let response = req.send().await?;
        self.handle_response(response).await
    }
    // Hashtag endpoints

    /// Get followed hashtags
    pub async fn get_followed_hashtags(&self) -> ApiResult<Vec<String>> {
        let url = format!("{}/hashtags/followed", self.base_url);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        let hashtags: Vec<serde_json::Value> = self.handle_response(response).await?;
        Ok(hashtags
            .into_iter()
            .filter_map(|h| h.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect())
    }

    /// Follow a hashtag
    pub async fn follow_hashtag(&self, name: String) -> ApiResult<()> {
        let url = self.build_url("/hashtags/follow");
        let request_body = serde_json::json!({ "name": name });
        let req = self.add_auth_header(self.client.post(&url).json(&request_body));
        let response = req.send().await?;
        let _: serde_json::Value = self.handle_response(response).await?;
        Ok(())
    }

    // Social endpoints

    /// Get following list
    pub async fn get_following_list(&self) -> ApiResult<Vec<SocialUserInfo>> {
        let url = format!("{}/social/following", self.base_url);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Get followers list
    pub async fn get_followers_list(&self) -> ApiResult<Vec<SocialUserInfo>> {
        let url = format!("{}/social/followers", self.base_url);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Get mutual friends list
    pub async fn get_mutual_friends_list(&self) -> ApiResult<Vec<SocialUserInfo>> {
        let url = format!("{}/social/mutual", self.base_url);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Search users by username
    pub async fn search_users(&self, query: String) -> ApiResult<Vec<UserSearchResult>> {
        let url = format!(
            "{}/users/search?q={}",
            self.base_url,
            urlencoding::encode(&query)
        );
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Get another user's profile with relationship status
    pub async fn get_user_profile_view(
        &self,
        user_id: Uuid,
    ) -> ApiResult<fido_types::UserProfileView> {
        let url = format!("{}/users/{}/profile-view", self.base_url, user_id);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Follow a user
    pub async fn follow_user(&self, user_id: Uuid) -> ApiResult<()> {
        let url = format!("{}/users/{}/follow", self.base_url, user_id);
        let req = self.add_auth_header(self.client.post(&url));
        req.send().await?.error_for_status()?;
        Ok(())
    }

    /// Unfollow a user
    pub async fn unfollow_user(&self, user_id: Uuid) -> ApiResult<()> {
        let url = format!("{}/users/{}/follow", self.base_url, user_id);
        let req = self.add_auth_header(self.client.delete(&url));
        req.send().await?.error_for_status()?;
        Ok(())
    }

    // OAuth endpoints

    /// Initiate GitHub Device Flow
    pub async fn github_device_flow(&self) -> ApiResult<GitHubDeviceFlowResponse> {
        let url = format!("{}/auth/github/device", self.base_url);
        let response = self.client.post(&url).send().await?;
        self.handle_response(response).await
    }

    /// Poll for Device Flow completion
    /// Returns Ok(LoginResponse) if authorized, Err with "authorization_pending" if still waiting
    pub async fn github_device_poll(&self, device_code: &str) -> ApiResult<LoginResponse> {
        let url = format!("{}/auth/github/device/poll", self.base_url);
        let payload = DevicePollRequest {
            device_code: device_code.to_string(),
        };
        let response = self.client.post(&url).json(&payload).send().await?;
        self.handle_response(response).await
    }

    /// Validate session token
    pub async fn validate_session(&self) -> ApiResult<ValidateSessionResponse> {
        let url = format!("{}/auth/validate", self.base_url);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Logout (invalidate session)
    pub async fn logout(&self, session_token: String) -> ApiResult<()> {
        let url = format!("{}/auth/logout", self.base_url);
        let response = self.client.post(&url).json(&session_token).send().await?;
        response.error_for_status()?;
        Ok(())
    }

    /// Get grouped unread notification counts.
    pub async fn get_notification_unread_counts(&self) -> ApiResult<Vec<NotificationUnreadCount>> {
        let url = self.build_url("/notifications/unread-count");
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// List recent notifications.
    pub async fn get_notifications(
        &self,
        limit: usize,
        offset: usize,
    ) -> ApiResult<Vec<Notification>> {
        let url = self.build_url(&format!("/notifications?limit={}&offset={}", limit, offset));
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Mark a single notification or all notifications as read.
    pub async fn mark_notifications_read(
        &self,
        notification_id: Option<Uuid>,
        all: bool,
    ) -> ApiResult<()> {
        let url = self.build_url("/notifications/mark-read");
        let request = MarkNotificationsReadRequest {
            notification_id,
            all,
        };
        let req = self.add_auth_header(self.client.post(&url).json(&request));
        let response = req.send().await?;
        let _: serde_json::Value = self.handle_response(response).await?;
        Ok(())
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        // Determine the appropriate base URL based on environment
        let base_url = if std::env::var("FIDO_WEB_MODE").is_ok() {
            // In web mode (containerized ttyd), connect to the API server on
            // the internal port nginx proxies to (see Dockerfile FIDO_SERVER_PORT).
            "http://127.0.0.1:4747".to_string()
        } else {
            // For local TUI client:
            // 1) explicit runtime override
            // 2) user-level config file (~/.fido/server_url)
            // 3) built-in public default
            std::env::var("FIDO_SERVER_URL")
                .ok()
                .filter(|url| !url.trim().is_empty())
                .or_else(read_server_url_from_config)
                .unwrap_or_else(|| DEFAULT_PUBLIC_SERVER_URL.to_string())
        };
        Self::new(base_url)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct SocialUserInfo {
    pub id: String,
    pub username: String,
    pub follower_count: usize,
    pub following_count: usize,
}

#[derive(Debug, serde::Deserialize)]
pub struct UserSearchResult {
    pub id: String,
    pub username: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitHubDeviceFlowResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct DevicePollRequest {
    pub device_code: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ValidateSessionResponse {
    pub user: fido_types::User,
    pub valid: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CommunityViewResponse {
    pub community: CommunityResponse,
    pub membership: Option<MembershipResponse>,
    pub member_count: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CommunityResponse {
    pub id: Uuid,
    pub owner: String,
    pub name: String,
    pub claimed_by: Option<Uuid>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MembershipResponse {
    pub role: fido_types::MembershipRole,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BrowseCommunityResponse {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    /// Every relationship that surfaced this repo (starred, owned, contributor).
    pub sources: Vec<fido_types::RepoSource>,
    pub community: Option<CommunityResponse>,
    pub membership: Option<MembershipResponse>,
}

#[derive(Debug, serde::Serialize)]
struct JoinCommunityRequest {
    owner: String,
    name: String,
}
