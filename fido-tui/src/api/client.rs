use reqwest::Client;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::{ApiError, ApiResult};
use fido_types::*;

/// API client for communicating with the Fido server
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    session_token: Option<String>,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            session_token: None,
        }
    }

    /// Set the session token for authenticated requests
    pub fn set_session_token(&mut self, token: Option<String>) {
        self.session_token = token;
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
    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> ApiResult<T> {
        let status = response.status();
        
        if status.is_success() {
            Ok(response.json().await?)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            // Clean up HTML error messages (e.g., from nginx 404 pages)
            let clean_error = if error_text.contains("<html>") || error_text.contains("<!DOCTYPE") {
                format!("Server returned {} error. Please check the server URL.", status.as_u16())
            } else {
                error_text
            };
            
            match status.as_u16() {
                404 => Err(ApiError::NotFound(clean_error)),
                401 => Err(ApiError::Unauthorized(clean_error)),
                400 => Err(ApiError::BadRequest(clean_error)),
                _ => Err(ApiError::Api(clean_error)),
            }
        }
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
    pub async fn get_posts(&self, limit: Option<i32>, sort: Option<String>, hashtag: Option<String>, username: Option<String>) -> ApiResult<Vec<Post>> {
        let mut url = format!("{}/posts", self.base_url);
        let mut params = vec![];
        
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(s) = sort {
            params.push(format!("sort={}", s));
        }
        if let Some(h) = hashtag {
            params.push(format!("hashtag={}", h));
        }
        if let Some(u) = username {
            params.push(format!("username={}", u));
        }
        
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }
        
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Create a new post
    pub async fn create_post(&self, content: String) -> ApiResult<Post> {
        let url = format!("{}/posts", self.base_url);
        let request = CreatePostRequest { content };
        let req = self.add_auth_header(self.client.post(&url).json(&request));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Vote on a post
    pub async fn vote_on_post(&self, post_id: Uuid, direction: String) -> ApiResult<serde_json::Value> {
        let url = format!("{}/posts/{}/vote", self.base_url, post_id);
        let request = VoteRequest { direction };
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

    /// Update a post
    pub async fn update_post(&self, post_id: Uuid, content: String) -> ApiResult<Post> {
        let url = format!("{}/posts/{}", self.base_url, post_id);
        let request = UpdatePostRequest { content };
        let req = self.add_auth_header(self.client.put(&url).json(&request));
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

    /// Get user profile view (for viewing any user's profile with relationship status)
    pub async fn get_user_profile_view(&self, user_id: String) -> ApiResult<fido_types::UserProfileView> {
        let url = format!("{}/users/{}/profile-view", self.base_url, user_id);
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
    pub async fn get_conversations(&self) -> ApiResult<Vec<serde_json::Value>> {
        let url = format!("{}/dms/conversations", self.base_url);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Get conversation with specific user
    pub async fn get_conversation(&self, user_id: Uuid) -> ApiResult<Vec<DirectMessage>> {
        let url = format!("{}/dms/conversations/{}", self.base_url, user_id);
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Send a direct message
    pub async fn send_message(&self, to_username: String, content: String) -> ApiResult<DirectMessage> {
        let url = format!("{}/dms", self.base_url);
        let request_body = SendMessageRequest { to_username, content };
        let req = self.add_auth_header(self.client.post(&url).json(&request_body));
        let response = req.send().await?;
        self.handle_response(response).await
    }

    /// Mark messages as read for a specific user
    pub async fn mark_messages_read(&self, user_id: Uuid) -> ApiResult<serde_json::Value> {
        let url = format!("{}/dms/mark-read/{}", self.base_url, user_id);
        let req = self.add_auth_header(self.client.post(&url));
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
        Ok(hashtags.into_iter().filter_map(|h| h.get("name").and_then(|n| n.as_str()).map(String::from)).collect())
    }

    /// Follow a hashtag
    pub async fn follow_hashtag(&self, name: String) -> ApiResult<()> {
        let url = format!("{}/hashtags/follow", self.base_url);
        let request_body = serde_json::json!({ "name": name });
        let req = self.add_auth_header(self.client.post(&url).json(&request_body));
        let response = req.send().await?;
        response.error_for_status()?;
        Ok(())
    }

    /// Unfollow a hashtag
    pub async fn unfollow_hashtag(&self, name: String) -> ApiResult<()> {
        let url = format!("{}/hashtags/follow/{}", self.base_url, name);
        let req = self.add_auth_header(self.client.delete(&url));
        let response = req.send().await?;
        response.error_for_status()?;
        Ok(())
    }

    /// Search hashtags
    pub async fn search_hashtags(&self, query: String) -> ApiResult<Vec<String>> {
        let url = format!("{}/hashtags/search?q={}", self.base_url, urlencoding::encode(&query));
        let req = self.client.get(&url);
        let response = req.send().await?;
        let hashtags: Vec<serde_json::Value> = self.handle_response(response).await?;
        Ok(hashtags.into_iter().filter_map(|h| h.get("name").and_then(|n| n.as_str()).map(String::from)).collect())
    }

    // Social endpoints

    /// Follow a user
    pub async fn follow_user(&self, user_id: String) -> ApiResult<()> {
        let url = format!("{}/users/{}/follow", self.base_url, user_id);
        let req = self.add_auth_header(self.client.post(&url));
        let response = req.send().await?;
        response.error_for_status()?;
        Ok(())
    }

    /// Unfollow a user
    pub async fn unfollow_user(&self, user_id: String) -> ApiResult<()> {
        let url = format!("{}/users/{}/follow", self.base_url, user_id);
        let req = self.add_auth_header(self.client.delete(&url));
        let response = req.send().await?;
        response.error_for_status()?;
        Ok(())
    }

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
        let url = format!("{}/users/search?q={}", self.base_url, urlencoding::encode(&query));
        let req = self.add_auth_header(self.client.get(&url));
        let response = req.send().await?;
        self.handle_response(response).await
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

    // Placeholder for future WebSocket integration
    // TODO: Add WebSocket support for real-time updates
    // pub async fn connect_websocket(&self) -> ApiResult<WebSocketStream> {
    //     // WebSocket connection logic will go here
    // }
}

impl Default for ApiClient {
    fn default() -> Self {
        // Determine the appropriate base URL based on environment
        let base_url = if std::env::var("FIDO_WEB_MODE").is_ok() {
            // In web mode (running on Fly.io), connect to localhost API server
            "http://127.0.0.1:3000".to_string()
        } else {
            // For local TUI client, check for override or use production URL
            std::env::var("FIDO_SERVER_URL")
                .unwrap_or_else(|_| "https://fido-social.fly.dev/api".to_string())
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
    pub expires_in: i64,
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

#[derive(Debug, serde::Deserialize)]
pub struct SessionPollResponse {
    pub session_token: Option<String>,
}
