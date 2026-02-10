use super::{ApiClient, ApiResult, MockBackend, VoteDirection};
use fido_types::*;
use uuid::Uuid;

/// Backend abstraction that can be either a real API client or a mock backend
#[derive(Clone)]
pub enum Backend {
    Api(ApiClient),
    Mock(MockBackend),
}

impl Backend {
    /// Create a new API backend
    pub fn api(base_url: impl Into<String>) -> Self {
        Backend::Api(ApiClient::new(base_url))
    }

    /// Create a new mock backend for demo mode
    pub fn mock() -> Self {
        Backend::Mock(MockBackend::new())
    }

    /// Set the session token for authenticated requests
    pub fn set_session_token(&mut self, token: Option<String>) {
        match self {
            Backend::Api(client) => client.set_session_token(token),
            Backend::Mock(_) => {
                // MockBackend handles session internally
            }
        }
    }

    // Authentication endpoints

    pub async fn get_test_users(&self) -> ApiResult<Vec<User>> {
        match self {
            Backend::Api(client) => client.get_test_users().await,
            Backend::Mock(mock) => mock.get_test_users().await,
        }
    }

    pub async fn login(&mut self, username: String) -> ApiResult<LoginResponse> {
        match self {
            Backend::Api(client) => client.login(username).await,
            Backend::Mock(mock) => mock.login(username).await,
        }
    }

    pub async fn validate_session(&self) -> ApiResult<super::client::ValidateSessionResponse> {
        match self {
            Backend::Api(client) => client.validate_session().await,
            Backend::Mock(_) => {
                // Mock backend doesn't support session validation
                Err(super::ApiError::Api(
                    "Session validation not supported in demo mode".to_string(),
                ))
            }
        }
    }

    pub async fn logout(&self, session_token: String) -> ApiResult<()> {
        match self {
            Backend::Api(client) => client.logout(session_token).await,
            Backend::Mock(_) => {
                // Mock backend doesn't need logout
                Ok(())
            }
        }
    }

    // Post endpoints

    pub async fn get_posts(
        &self,
        limit: Option<i32>,
        sort: Option<String>,
        hashtag: Option<String>,
        username: Option<String>,
    ) -> ApiResult<Vec<Post>> {
        match self {
            Backend::Api(client) => client.get_posts(limit, sort, hashtag, username).await,
            Backend::Mock(mock) => {
                mock.get_posts(limit.map(|l| l as usize), sort, hashtag, username)
                    .await
            }
        }
    }

    pub async fn create_post(&mut self, content: String) -> ApiResult<Post> {
        match self {
            Backend::Api(client) => client.create_post(content).await,
            Backend::Mock(mock) => mock.create_post(content).await,
        }
    }

    pub async fn vote_on_post(
        &mut self,
        post_id: Uuid,
        direction: VoteDirection,
    ) -> ApiResult<serde_json::Value> {
        match self {
            Backend::Api(client) => client.vote_on_post(post_id, direction).await,
            Backend::Mock(mock) => {
                mock.vote_on_post(post_id, direction.to_string()).await?;
                Ok(serde_json::json!({}))
            }
        }
    }

    pub async fn get_post_by_id(&self, post_id: Uuid) -> ApiResult<Post> {
        match self {
            Backend::Api(client) => client.get_post_by_id(post_id).await,
            Backend::Mock(mock) => {
                // MockBackend doesn't have get_post_by_id, so we'll get all posts and filter
                let posts = mock.get_posts(None, None, None, None).await?;
                posts
                    .into_iter()
                    .find(|p| p.id == post_id)
                    .ok_or_else(|| super::ApiError::NotFound("Post not found".to_string()))
            }
        }
    }

    pub async fn get_replies(&self, post_id: Uuid) -> ApiResult<Vec<Post>> {
        match self {
            Backend::Api(client) => client.get_replies(post_id).await,
            Backend::Mock(mock) => mock.get_replies(post_id).await,
        }
    }

    pub async fn create_reply(&mut self, post_id: Uuid, content: String) -> ApiResult<Post> {
        match self {
            Backend::Api(client) => client.create_reply(post_id, content).await,
            Backend::Mock(mock) => mock.create_reply(post_id, content).await,
        }
    }

    pub async fn delete_post(&mut self, post_id: Uuid) -> ApiResult<serde_json::Value> {
        match self {
            Backend::Api(client) => client.delete_post(post_id).await,
            Backend::Mock(_) => {
                // MockBackend doesn't support delete_post
                Err(super::ApiError::Api(
                    "Post deletion not supported in demo mode".to_string(),
                ))
            }
        }
    }

    // Profile endpoints

    pub async fn get_profile(&self, user_id: Uuid) -> ApiResult<UserProfile> {
        match self {
            Backend::Api(client) => client.get_profile(user_id).await,
            Backend::Mock(mock) => mock.get_profile(user_id).await,
        }
    }

    pub async fn update_bio(&mut self, user_id: Uuid, bio: String) -> ApiResult<serde_json::Value> {
        match self {
            Backend::Api(client) => client.update_bio(user_id, bio).await,
            Backend::Mock(mock) => {
                mock.update_bio(bio).await?;
                Ok(serde_json::json!({}))
            }
        }
    }

    // Direct message endpoints

    pub async fn get_conversations(&self) -> ApiResult<Vec<serde_json::Value>> {
        match self {
            Backend::Api(client) => client.get_conversations().await,
            Backend::Mock(mock) => mock.get_conversations().await,
        }
    }

    pub async fn get_conversation(&self, user_id: Uuid) -> ApiResult<Vec<DirectMessage>> {
        match self {
            Backend::Api(client) => client.get_conversation(user_id).await,
            Backend::Mock(mock) => mock.get_conversation(user_id).await,
        }
    }

    pub async fn send_message(
        &mut self,
        to_username: String,
        content: String,
    ) -> ApiResult<DirectMessage> {
        match self {
            Backend::Api(client) => client.send_message(to_username, content).await,
            Backend::Mock(mock) => mock.send_message(to_username, content).await,
        }
    }

    // Configuration endpoints

    pub async fn get_config(&self) -> ApiResult<UserConfig> {
        match self {
            Backend::Api(client) => client.get_config().await,
            Backend::Mock(mock) => mock.get_config().await,
        }
    }

    pub async fn update_config(&mut self, request: UpdateConfigRequest) -> ApiResult<UserConfig> {
        match self {
            Backend::Api(client) => client.update_config(request).await,
            Backend::Mock(mock) => {
                mock.update_config(
                    request.color_scheme,
                    request.sort_order,
                    request.max_posts_display,
                    request.emoji_enabled,
                )
                .await
            }
        }
    }

    // Hashtag endpoints

    pub async fn get_followed_hashtags(&self) -> ApiResult<Vec<String>> {
        match self {
            Backend::Api(client) => client.get_followed_hashtags().await,
            Backend::Mock(mock) => mock.get_followed_hashtags().await,
        }
    }

    pub async fn follow_hashtag(&mut self, name: String) -> ApiResult<()> {
        match self {
            Backend::Api(client) => client.follow_hashtag(name).await,
            Backend::Mock(mock) => mock.follow_hashtag(name).await,
        }
    }

    // Social endpoints

    pub async fn get_following_list(&self) -> ApiResult<Vec<super::client::SocialUserInfo>> {
        match self {
            Backend::Api(client) => client.get_following_list().await,
            Backend::Mock(_) => {
                // MockBackend doesn't support social features
                Ok(Vec::new())
            }
        }
    }

    pub async fn get_followers_list(&self) -> ApiResult<Vec<super::client::SocialUserInfo>> {
        match self {
            Backend::Api(client) => client.get_followers_list().await,
            Backend::Mock(_) => {
                // MockBackend doesn't support social features
                Ok(Vec::new())
            }
        }
    }

    pub async fn get_mutual_friends_list(&self) -> ApiResult<Vec<super::client::SocialUserInfo>> {
        match self {
            Backend::Api(client) => client.get_mutual_friends_list().await,
            Backend::Mock(_) => {
                // MockBackend doesn't support social features
                Ok(Vec::new())
            }
        }
    }

    pub async fn search_users(
        &self,
        query: String,
    ) -> ApiResult<Vec<super::client::UserSearchResult>> {
        match self {
            Backend::Api(client) => client.search_users(query).await,
            Backend::Mock(_) => {
                // MockBackend doesn't support user search
                Ok(Vec::new())
            }
        }
    }

    // OAuth endpoints

    pub async fn github_device_flow(&self) -> ApiResult<super::client::GitHubDeviceFlowResponse> {
        match self {
            Backend::Api(client) => client.github_device_flow().await,
            Backend::Mock(_) => {
                // MockBackend doesn't support GitHub OAuth
                Err(super::ApiError::Api(
                    "GitHub OAuth not supported in demo mode".to_string(),
                ))
            }
        }
    }

    pub async fn github_device_poll(&self, device_code: &str) -> ApiResult<LoginResponse> {
        match self {
            Backend::Api(client) => client.github_device_poll(device_code).await,
            Backend::Mock(_) => {
                // MockBackend doesn't support GitHub OAuth
                Err(super::ApiError::Api(
                    "GitHub OAuth not supported in demo mode".to_string(),
                ))
            }
        }
    }
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Api(ApiClient::default())
    }
}
