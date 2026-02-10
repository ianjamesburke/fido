use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::sample_data::{create_sample_conversations, create_sample_posts, create_test_users};
use super::{ApiError, ApiResult};
use fido_types::enums::{ColorScheme, SortOrder};
use fido_types::*;

/// Mock backend for demo mode - provides in-memory data storage
#[derive(Clone)]
pub struct MockBackend {
    data: Arc<Mutex<MockData>>,
    current_user: Option<User>,
    session_token: Option<String>,
}

/// In-memory data store for demo mode
struct MockData {
    users: Vec<User>,
    posts: Vec<Post>,
    messages: Vec<DirectMessage>,
    votes: Vec<(Uuid, Uuid, String)>, // (user_id, post_id, direction)
    followed_hashtags: Vec<(Uuid, String)>, // (user_id, hashtag)
    configs: Vec<(Uuid, UserConfig)>,
}

/// Helper enum for vote actions to avoid borrow checker issues
enum VoteAction {
    Remove(usize, String),         // (index, direction)
    Change(usize, String, String), // (index, old_direction, new_direction)
    Add(String),                   // direction
}

impl MockBackend {
    /// Create a new MockBackend with sample data
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(MockData::with_sample_data())),
            current_user: None,
            session_token: None,
        }
    }

    // Authentication methods

    /// Get list of test users available for demo login
    pub async fn get_test_users(&self) -> ApiResult<Vec<User>> {
        let data = self.data.lock().unwrap();
        Ok(data.users.clone())
    }

    /// Login with username - finds user and generates demo session token
    pub async fn login(&mut self, username: String) -> ApiResult<LoginResponse> {
        let data = self.data.lock().unwrap();

        let user = data
            .users
            .iter()
            .find(|u| u.username == username)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

        // Generate demo session token
        let token = format!("demo-token-{}", Uuid::new_v4());

        // Store current user and session
        drop(data); // Release lock before modifying self
        self.current_user = Some(user.clone());
        self.session_token = Some(token.clone());

        Ok(LoginResponse {
            user,
            session_token: token,
        })
    }

    /// Helper to get current user or return auth error
    fn require_auth(&self) -> ApiResult<&User> {
        self.current_user
            .as_ref()
            .ok_or_else(|| ApiError::Unauthorized("Please log in to continue".to_string()))
    }

    // Post operations

    /// Get posts with sorting and filtering
    pub async fn get_posts(
        &self,
        limit: Option<usize>,
        sort: Option<String>,
        hashtag: Option<String>,
        username: Option<String>,
    ) -> ApiResult<Vec<Post>> {
        let data = self.data.lock().unwrap();

        // Start with all top-level posts (no parent_post_id)
        let mut posts: Vec<Post> = data
            .posts
            .iter()
            .filter(|p| p.parent_post_id.is_none())
            .cloned()
            .collect();

        // Apply hashtag filter if specified
        if let Some(ref tag) = hashtag {
            let tag_lower = tag.to_lowercase();
            posts.retain(|p| p.hashtags.iter().any(|h| h.to_lowercase() == tag_lower));
        }

        // Apply username filter if specified
        if let Some(ref user) = username {
            posts.retain(|p| p.author_username == *user);
        }

        // Apply user's vote status if logged in
        if let Some(ref current_user) = self.current_user {
            for post in &mut posts {
                // Find if user has voted on this post
                if let Some((_, _, direction)) = data
                    .votes
                    .iter()
                    .find(|(uid, pid, _)| uid == &current_user.id && pid == &post.id)
                {
                    post.user_vote = Some(direction.clone());
                }
            }
        }

        // Sort posts
        let sort_order = sort.as_deref().unwrap_or("Newest");
        match sort_order {
            "Newest" => {
                posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
            "Popular" => {
                posts.sort_by(|a, b| {
                    let score_a = a.upvotes;
                    let score_b = b.upvotes;
                    score_b
                        .cmp(&score_a)
                        .then_with(|| b.created_at.cmp(&a.created_at))
                });
            }
            "Controversial" => {
                posts.sort_by(|a, b| {
                    let controversy_a = (a.upvotes - a.downvotes).abs();
                    let controversy_b = (b.upvotes - b.downvotes).abs();
                    controversy_a
                        .cmp(&controversy_b)
                        .then_with(|| b.created_at.cmp(&a.created_at))
                });
            }
            _ => {
                // Default to newest
                posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
        }

        // Apply limit if specified
        if let Some(lim) = limit {
            posts.truncate(lim);
        }

        Ok(posts)
    }

    /// Create a new post with hashtag extraction
    pub async fn create_post(&mut self, content: String) -> ApiResult<Post> {
        // Require authentication
        let current_user = self.require_auth()?.clone();

        // Validate content
        if content.trim().is_empty() {
            return Err(ApiError::BadRequest(
                "Post content cannot be empty".to_string(),
            ));
        }

        // Extract hashtags from content (words starting with #)
        let hashtags = extract_hashtags(&content);

        // Create new post
        let post = Post {
            id: Uuid::new_v4(),
            author_id: current_user.id,
            author_username: current_user.username.clone(),
            content,
            created_at: chrono::Utc::now(),
            upvotes: 0,
            downvotes: 0,
            hashtags,
            user_vote: None,
            parent_post_id: None,
            reply_count: 0,
            reply_to_user_id: None,
            reply_to_username: None,
        };

        // Add to data store
        let mut data = self.data.lock().unwrap();
        data.posts.push(post.clone());

        Ok(post)
    }

    /// Vote on a post (upvote or downvote)
    pub async fn vote_on_post(&mut self, post_id: Uuid, direction: String) -> ApiResult<()> {
        // Require authentication
        let current_user = self.require_auth()?.clone();

        // Validate direction
        if direction != "up" && direction != "down" {
            return Err(ApiError::BadRequest(
                "Vote direction must be 'up' or 'down'".to_string(),
            ));
        }

        let mut data = self.data.lock().unwrap();

        // Check if post exists
        if !data.posts.iter().any(|p| p.id == post_id) {
            return Err(ApiError::NotFound("Post not found".to_string()));
        }

        // Determine what action to take based on existing vote
        let vote_action = if let Some(existing_vote_idx) = data
            .votes
            .iter()
            .position(|(uid, pid, _)| uid == &current_user.id && pid == &post_id)
        {
            let (_, _, existing_direction) = &data.votes[existing_vote_idx];

            if existing_direction == &direction {
                // Same direction - toggle off (remove vote)
                VoteAction::Remove(existing_vote_idx, direction.clone())
            } else {
                // Different direction - change vote
                VoteAction::Change(
                    existing_vote_idx,
                    existing_direction.clone(),
                    direction.clone(),
                )
            }
        } else {
            // No existing vote - add new vote
            VoteAction::Add(direction.clone())
        };

        // Apply the vote action
        match vote_action {
            VoteAction::Remove(idx, dir) => {
                data.votes.remove(idx);
                // Update post counts
                if let Some(post) = data.posts.iter_mut().find(|p| p.id == post_id) {
                    if dir == "up" {
                        post.upvotes -= 1;
                    } else {
                        post.downvotes -= 1;
                    }
                }
            }
            VoteAction::Change(idx, _old_dir, new_dir) => {
                data.votes[idx].2 = new_dir.clone();
                // Update post counts (remove old, add new)
                if let Some(post) = data.posts.iter_mut().find(|p| p.id == post_id) {
                    if new_dir == "up" {
                        post.downvotes -= 1;
                        post.upvotes += 1;
                    } else {
                        post.upvotes -= 1;
                        post.downvotes += 1;
                    }
                }
            }
            VoteAction::Add(dir) => {
                data.votes.push((current_user.id, post_id, dir.clone()));
                // Update post counts
                if let Some(post) = data.posts.iter_mut().find(|p| p.id == post_id) {
                    if dir == "up" {
                        post.upvotes += 1;
                    } else {
                        post.downvotes += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get replies for a specific post
    pub async fn get_replies(&self, post_id: Uuid) -> ApiResult<Vec<Post>> {
        let data = self.data.lock().unwrap();

        // Filter posts by parent_post_id
        let mut replies: Vec<Post> = data
            .posts
            .iter()
            .filter(|p| p.parent_post_id == Some(post_id))
            .cloned()
            .collect();

        // Apply user's vote status if logged in
        if let Some(ref current_user) = self.current_user {
            for reply in &mut replies {
                // Find if user has voted on this reply
                if let Some((_, _, direction)) = data
                    .votes
                    .iter()
                    .find(|(uid, pid, _)| uid == &current_user.id && pid == &reply.id)
                {
                    reply.user_vote = Some(direction.clone());
                }
            }
        }

        // Sort by created_at (oldest first for chronological reading)
        replies.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok(replies)
    }

    /// Create a reply to a post
    pub async fn create_reply(&mut self, parent_post_id: Uuid, content: String) -> ApiResult<Post> {
        // Require authentication
        let current_user = self.require_auth()?.clone();

        // Validate content
        if content.trim().is_empty() {
            return Err(ApiError::BadRequest(
                "Reply content cannot be empty".to_string(),
            ));
        }

        let mut data = self.data.lock().unwrap();

        // Find the parent post
        let parent_post = data
            .posts
            .iter()
            .find(|p| p.id == parent_post_id)
            .ok_or_else(|| ApiError::NotFound("Parent post not found".to_string()))?;

        let reply_to_user_id = parent_post.author_id;
        let reply_to_username = parent_post.author_username.clone();

        // Extract hashtags from content
        let hashtags = extract_hashtags(&content);

        // Create reply post
        let reply = Post {
            id: Uuid::new_v4(),
            author_id: current_user.id,
            author_username: current_user.username.clone(),
            content,
            created_at: chrono::Utc::now(),
            upvotes: 0,
            downvotes: 0,
            hashtags,
            user_vote: None,
            parent_post_id: Some(parent_post_id),
            reply_count: 0,
            reply_to_user_id: Some(reply_to_user_id),
            reply_to_username: Some(reply_to_username),
        };

        // Add reply to data store
        data.posts.push(reply.clone());

        // Increment parent post's reply_count
        if let Some(parent) = data.posts.iter_mut().find(|p| p.id == parent_post_id) {
            parent.reply_count += 1;
        }

        Ok(reply)
    }

    // Direct message operations

    /// Get conversations list - groups messages by conversation partner
    pub async fn get_conversations(&self) -> ApiResult<Vec<serde_json::Value>> {
        // Require authentication
        let current_user = self.require_auth()?;
        let current_user_id = current_user.id;

        let data = self.data.lock().unwrap();

        // Group messages by conversation partner
        use std::collections::HashMap;
        let mut conversations: HashMap<Uuid, Vec<&DirectMessage>> = HashMap::new();

        for message in &data.messages {
            // Determine the other user in the conversation
            let other_user_id = if message.from_user_id == current_user_id {
                message.to_user_id
            } else if message.to_user_id == current_user_id {
                message.from_user_id
            } else {
                // Message doesn't involve current user
                continue;
            };

            conversations
                .entry(other_user_id)
                .or_insert_with(Vec::new)
                .push(message);
        }

        // Build conversation summaries
        let mut result = Vec::new();

        for (other_user_id, messages) in conversations {
            // Find the other user
            let other_user = data
                .users
                .iter()
                .find(|u| u.id == other_user_id)
                .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

            // Find the most recent message
            let last_message = messages
                .iter()
                .max_by_key(|m| m.created_at)
                .ok_or_else(|| ApiError::NotFound("No messages found".to_string()))?;

            // Count unread messages (messages to current user that are unread)
            let unread_count = messages
                .iter()
                .filter(|m| m.to_user_id == current_user_id && !m.is_read)
                .count();

            // Create conversation summary
            let conversation = serde_json::json!({
                "other_user_id": other_user_id,
                "other_username": other_user.username,
                "last_message": last_message.content,
                "last_message_time": last_message.created_at,
                "unread_count": unread_count,
            });

            result.push(conversation);
        }

        // Sort by most recent message (descending)
        result.sort_by(|a, b| {
            let time_a = a
                .get("last_message_time")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));
            let time_b = b
                .get("last_message_time")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));

            time_b.cmp(&time_a) // Descending order (most recent first)
        });

        Ok(result)
    }

    /// Get conversation with a specific user - returns messages between current user and specified user
    pub async fn get_conversation(&self, user_id: Uuid) -> ApiResult<Vec<DirectMessage>> {
        // Require authentication
        let current_user = self.require_auth()?;
        let current_user_id = current_user.id;

        let data = self.data.lock().unwrap();

        // Check if the other user exists
        if !data.users.iter().any(|u| u.id == user_id) {
            return Err(ApiError::NotFound("User not found".to_string()));
        }

        // Get all messages between current user and specified user
        let mut messages: Vec<DirectMessage> = data
            .messages
            .iter()
            .filter(|m| {
                (m.from_user_id == current_user_id && m.to_user_id == user_id)
                    || (m.from_user_id == user_id && m.to_user_id == current_user_id)
            })
            .cloned()
            .collect();

        // Sort by created_at (ascending - chronological order)
        messages.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok(messages)
    }

    /// Send a direct message to another user
    pub async fn send_message(
        &mut self,
        to_username: String,
        content: String,
    ) -> ApiResult<DirectMessage> {
        // Require authentication
        let current_user = self.require_auth()?.clone();

        // Validate content
        if content.trim().is_empty() {
            return Err(ApiError::BadRequest(
                "Message content cannot be empty".to_string(),
            ));
        }

        let mut data = self.data.lock().unwrap();

        // Find the recipient user by username
        let to_user = data
            .users
            .iter()
            .find(|u| u.username == to_username)
            .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", to_username)))?;

        let to_user_id = to_user.id;
        let to_user_username = to_user.username.clone();

        // Create new direct message
        let message = DirectMessage {
            id: Uuid::new_v4(),
            from_user_id: current_user.id,
            from_username: current_user.username.clone(),
            to_user_id: to_user_id,
            to_username: to_user_username,
            content,
            created_at: chrono::Utc::now(),
            is_read: false,
        };

        // Add to data store
        data.messages.push(message.clone());

        Ok(message)
    }

    // Profile operations

    /// Get user profile by user_id
    pub async fn get_profile(&self, user_id: Uuid) -> ApiResult<UserProfile> {
        let data = self.data.lock().unwrap();

        // Find the user
        let user = data
            .users
            .iter()
            .find(|u| u.id == user_id)
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

        // Calculate karma (sum of upvotes - downvotes on user's posts)
        let karma: i32 = data
            .posts
            .iter()
            .filter(|p| p.author_id == user_id)
            .map(|p| p.upvotes - p.downvotes)
            .sum();

        // Count posts by user
        let post_count = data
            .posts
            .iter()
            .filter(|p| p.author_id == user_id && p.parent_post_id.is_none())
            .count() as i32;

        // Get recent hashtags from user's posts (last 10 unique hashtags)
        let mut recent_hashtags: Vec<String> = Vec::new();
        let mut user_posts: Vec<&Post> = data
            .posts
            .iter()
            .filter(|p| p.author_id == user_id)
            .collect();
        user_posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        for post in user_posts {
            for hashtag in &post.hashtags {
                if !recent_hashtags.contains(hashtag) {
                    recent_hashtags.push(hashtag.clone());
                    if recent_hashtags.len() >= 10 {
                        break;
                    }
                }
            }
            if recent_hashtags.len() >= 10 {
                break;
            }
        }

        Ok(UserProfile {
            user_id: user.id,
            username: user.username.clone(),
            bio: user.bio.clone(),
            karma,
            post_count,
            join_date: user.join_date,
            recent_hashtags,
        })
    }

    /// Update user's bio
    pub async fn update_bio(&mut self, bio: String) -> ApiResult<()> {
        // Require authentication
        let current_user = self.require_auth()?.clone();

        let mut data = self.data.lock().unwrap();

        // Find and update the user's bio
        if let Some(user) = data.users.iter_mut().find(|u| u.id == current_user.id) {
            user.bio = Some(bio.clone());

            // Also update current_user in self
            drop(data);
            if let Some(ref mut cu) = self.current_user {
                cu.bio = Some(bio);
            }

            Ok(())
        } else {
            Err(ApiError::NotFound("User not found".to_string()))
        }
    }

    // Config operations

    /// Get user config - returns existing config or creates default
    pub async fn get_config(&self) -> ApiResult<UserConfig> {
        // Require authentication
        let current_user = self.require_auth()?;
        let user_id = current_user.id;

        let data = self.data.lock().unwrap();

        // Try to find existing config
        if let Some((_, config)) = data.configs.iter().find(|(uid, _)| uid == &user_id) {
            Ok(config.clone())
        } else {
            // Return default config with user_id set
            Ok(UserConfig {
                user_id,
                ..UserConfig::default()
            })
        }
    }

    /// Update user config
    pub async fn update_config(
        &mut self,
        color_scheme: Option<String>,
        sort_order: Option<String>,
        max_posts_display: Option<i32>,
        emoji_enabled: Option<bool>,
    ) -> ApiResult<UserConfig> {
        // Require authentication
        let current_user = self.require_auth()?;
        let user_id = current_user.id;

        let mut data = self.data.lock().unwrap();

        // Get existing config or create default
        let mut config = data
            .configs
            .iter()
            .find(|(uid, _)| uid == &user_id)
            .map(|(_, c)| c.clone())
            .unwrap_or_else(|| UserConfig {
                user_id,
                ..UserConfig::default()
            });

        // Update fields if provided
        if let Some(cs) = color_scheme {
            config.color_scheme = ColorScheme::parse(&cs)
                .ok_or_else(|| ApiError::BadRequest(format!("Invalid color scheme: {}", cs)))?;
        }

        if let Some(so) = sort_order {
            config.sort_order = SortOrder::parse(&so)
                .ok_or_else(|| ApiError::BadRequest(format!("Invalid sort order: {}", so)))?;
        }

        if let Some(mpd) = max_posts_display {
            if mpd < 1 || mpd > 100 {
                return Err(ApiError::BadRequest(
                    "max_posts_display must be between 1 and 100".to_string(),
                ));
            }
            config.max_posts_display = mpd;
        }

        if let Some(ee) = emoji_enabled {
            config.emoji_enabled = ee;
        }

        // Update or insert config
        if let Some(pos) = data.configs.iter().position(|(uid, _)| uid == &user_id) {
            data.configs[pos] = (user_id, config.clone());
        } else {
            data.configs.push((user_id, config.clone()));
        }

        Ok(config)
    }

    // Hashtag operations

    /// Get followed hashtags for current user
    pub async fn get_followed_hashtags(&self) -> ApiResult<Vec<String>> {
        // Require authentication
        let current_user = self.require_auth()?;
        let user_id = current_user.id;

        let data = self.data.lock().unwrap();

        // Get all hashtags followed by this user
        let hashtags: Vec<String> = data
            .followed_hashtags
            .iter()
            .filter(|(uid, _)| uid == &user_id)
            .map(|(_, tag)| tag.clone())
            .collect();

        Ok(hashtags)
    }

    /// Follow a hashtag
    pub async fn follow_hashtag(&mut self, hashtag: String) -> ApiResult<()> {
        // Require authentication
        let current_user = self.require_auth()?;
        let user_id = current_user.id;

        // Normalize hashtag (lowercase, remove # if present)
        let normalized_tag = hashtag.trim_start_matches('#').to_lowercase();

        if normalized_tag.is_empty() {
            return Err(ApiError::BadRequest("Hashtag cannot be empty".to_string()));
        }

        let mut data = self.data.lock().unwrap();

        // Check if already following
        if data
            .followed_hashtags
            .iter()
            .any(|(uid, tag)| uid == &user_id && tag == &normalized_tag)
        {
            return Err(ApiError::BadRequest(
                "Already following this hashtag".to_string(),
            ));
        }

        // Add to followed hashtags
        data.followed_hashtags.push((user_id, normalized_tag));

        Ok(())
    }
}

impl MockData {
    /// Initialize with sample data for demo
    fn with_sample_data() -> Self {
        let users = create_test_users();
        let posts = create_sample_posts(&users);
        let messages = create_sample_conversations(&users);

        Self {
            users,
            posts,
            messages,
            votes: Vec::new(),
            followed_hashtags: Vec::new(),
            configs: Vec::new(),
        }
    }
}

/// Extract hashtags from content (words starting with #)
fn extract_hashtags(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .filter_map(|word| {
            // Check if word starts with #
            if word.starts_with('#') {
                // Remove the # and any trailing punctuation
                let tag = word[1..].trim_end_matches(|c: char| !c.is_alphanumeric());
                if !tag.is_empty() {
                    Some(tag.to_lowercase())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}
