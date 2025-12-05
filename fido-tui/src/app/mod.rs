use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use fido_types::Post;
use ratatui::style::Style;
use ratatui::widgets::ListState;
use std::time::{Duration, Instant};
use tui_textarea::TextArea;
use uuid::Uuid;

use crate::api::ApiClient;

pub mod state;
pub use state::*;
pub mod handlers;

impl App {
    pub fn new() -> Self {
        let config_manager =
            crate::config::ConfigManager::new().expect("Failed to initialize config manager");
        let instance_id = crate::config::ConfigManager::generate_instance_id();

        // Clean up old sessions on startup
        let _ = config_manager.cleanup_old_sessions();

        Self {
            running: true,
            current_screen: Screen::Auth,
            api_client: ApiClient::default(),
            auth_state: AuthState {
                test_users: Vec::new(),
                selected_index: 0,
                loading: false,
                error: None,
                current_user: None,
                show_github_option: true,
                github_auth_in_progress: false,
                github_device_code: None,
                github_user_code: None,
                github_verification_uri: None,
                github_poll_interval: None,
                github_auth_start_time: None,
            },
            current_tab: Tab::Posts,
            posts_state: PostsState {
                posts: Vec::new(),
                list_state: ListState::default(),
                loading: false,
                error: None,
                message: None,
                show_new_post_modal: false,
                new_post_content: String::new(),
                pending_load: false,
                current_filter: PostFilter::All,
                show_filter_modal: false,
                filter_modal_state: FilterModalState {
                    selected_tab: FilterTab::All,
                    hashtag_list: Vec::new(),
                    user_list: Vec::new(),
                    selected_index: 0,
                    search_input: String::new(),
                    search_mode: false,
                    search_results: Vec::new(),
                    checked_hashtags: Vec::new(),
                    checked_users: Vec::new(),
                    show_add_hashtag_input: false,
                    add_hashtag_input: String::new(),
                },
                sort_order: "Newest".to_string(),
                at_end_of_feed: false,
            },
            profile_state: ProfileState {
                profile: None,
                user_posts: Vec::new(),
                list_state: ListState::default(),
                loading: false,
                error: None,
                show_edit_bio_modal: false,
                edit_bio_content: String::new(),
                edit_bio_cursor_position: 0,
            },
            dms_state: DMsState {
                conversations: Vec::new(),
                selected_conversation_index: None, // No conversation selected by default
                messages: Vec::new(),
                loading: false,
                error: None,
                message_input: String::new(),
                message_textarea: {
                    let mut textarea = TextArea::default();
                    textarea.set_cursor_line_style(Style::default());
                    textarea.set_style(Style::default());
                    // Enable hard tab indent for better wrapping behavior
                    textarea.set_hard_tab_indent(true);
                    textarea
                },
                messages_scroll_offset: 0,
                show_new_conversation_modal: false,
                new_conversation_username: String::new(),
                pending_conversation_username: None,
                unread_counts: std::collections::HashMap::new(),
                current_conversation_user: None,
                needs_message_load: false,
                show_dm_error_modal: false,
                dm_error_message: String::new(),
                failed_username: None,
                available_mutual_friends: Vec::new(),
            },
            settings_state: SettingsState {
                config: None,
                original_config: None,
                original_max_posts_input: String::new(),
                loading: false,
                error: None,
                selected_field: SettingsField::ColorScheme,
                max_posts_input: String::new(),
                has_unsaved_changes: false,
                show_save_confirmation: false,
                pending_tab: None,
            },
            post_detail_state: None,
            viewing_post_detail: false,
            config_manager,
            instance_id,
            show_help: false,
            input_mode: InputMode::Navigation,
            composer_state: ComposerState::new(),
            friends_state: FriendsState {
                show_friends_modal: false,
                selected_tab: SocialTab::Following,
                following: Vec::new(),
                followers: Vec::new(),
                mutual_friends: Vec::new(),
                selected_index: 0,
                search_query: String::new(),
                search_mode: false,
                error: None,
                loading: false,
                return_to_modal_after_profile: false,
            },
            hashtags_state: HashtagsState {
                hashtags: Vec::new(),
                show_hashtags_modal: false,
                show_add_hashtag_input: false,
                add_hashtag_name: String::new(),
                selected_hashtag: 0,
                error: None,
                loading: false,
                show_unfollow_confirmation: false,
                hashtag_to_unfollow: None,
            },
            user_profile_view: None,
            log_config: crate::logging::LogConfig::default(),
        }
    }

    /// Toggle help modal
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        let next = self.current_tab.next();
        self.try_switch_tab(next);
    }

    /// Switch to previous tab
    pub fn previous_tab(&mut self) {
        let prev = self.current_tab.previous();
        self.try_switch_tab(prev);
    }

    /// Try to switch tab, checking for unsaved changes
    fn try_switch_tab(&mut self, new_tab: Tab) {
        // If we're leaving settings with unsaved changes, show confirmation
        if self.current_tab == Tab::Settings && self.settings_state.has_unsaved_changes {
            self.settings_state.show_save_confirmation = true;
            self.settings_state.pending_tab = Some(new_tab);
        } else {
            self.current_tab = new_tab;
        }
    }

    /// Confirm tab switch or logout without saving
    pub fn confirm_discard_changes(&mut self) {
        self.settings_state.has_unsaved_changes = false;
        self.settings_state.show_save_confirmation = false;

        if let Some(pending_tab) = self.settings_state.pending_tab.take() {
            // Switch to pending tab
            self.current_tab = pending_tab;
        } else {
            // No pending tab means logout/exit was requested
            // Clear session file
            if let Err(e) = self.config_manager.delete_session(&self.instance_id) {
                eprintln!("Warning: Failed to delete session: {}", e);
            }

            // Reset app state
            self.auth_state.current_user = None;
            self.current_screen = Screen::Auth;
            self.posts_state.posts.clear();
            self.profile_state.profile = None;
            self.dms_state.conversations.clear();
            self.dms_state.messages.clear();
        }
    }

    /// Cancel tab switch
    pub fn cancel_tab_switch(&mut self) {
        self.settings_state.show_save_confirmation = false;
        self.settings_state.pending_tab = None;
    }

    /// Clear expired messages (auto-clear after 3 seconds)
    pub fn clear_expired_messages(&mut self) {
        let now = std::time::Instant::now();
        let duration = std::time::Duration::from_secs(3);

        // Clear posts state message if expired
        if let Some((_, timestamp)) = &self.posts_state.message {
            if now.duration_since(*timestamp) > duration {
                self.posts_state.message = None;
            }
        }

        // Clear post detail state message if expired
        if let Some(detail_state) = &mut self.post_detail_state {
            if let Some((_, timestamp)) = &detail_state.message {
                if now.duration_since(*timestamp) > duration {
                    detail_state.message = None;
                }
            }
        }
    }

    /// Check if we need to load data when switching tabs
    pub fn needs_tab_data_load(&self) -> bool {
        matches!(self.current_tab, Tab::Profile | Tab::DMs | Tab::Settings)
    }

    /// Logout and clear session
    pub async fn logout(&mut self) -> Result<()> {
        // Check for unsaved changes in Settings
        if self.current_tab == Tab::Settings && self.settings_state.has_unsaved_changes {
            self.settings_state.show_save_confirmation = true;
            self.settings_state.pending_tab = None; // None indicates logout
            return Ok(());
        }

        // Call server logout endpoint to invalidate session (best effort)
        // We don't fail if this errors since we'll clear local session anyway
        if let Ok(session_store) = crate::session::SessionStore::new() {
            if let Ok(Some(token)) = session_store.load() {
                let _ = self.api_client.logout(token).await;
            }
            
            // Delete local session file
            if let Err(e) = session_store.delete() {
                log::warn!("Failed to delete session file: {}", e);
            }
        }

        // Also clear old config_manager session for backwards compatibility
        if let Err(e) = self.config_manager.delete_session(&self.instance_id) {
            log::warn!("Failed to delete config_manager session: {}", e);
        }

        // Reset app state
        self.auth_state.current_user = None;
        self.current_screen = Screen::Auth;
        self.posts_state.posts.clear();
        self.profile_state.profile = None;
        self.dms_state.conversations.clear();
        self.dms_state.messages.clear();
        
        // Reset GitHub Device Flow state
        self.auth_state.github_auth_in_progress = false;
        self.auth_state.github_device_code = None;
        self.auth_state.github_user_code = None;
        self.auth_state.github_verification_uri = None;
        self.auth_state.github_poll_interval = None;
        self.auth_state.github_auth_start_time = None;
        self.auth_state.error = None;

        Ok(())
    }

    /// Load test users from API
    pub async fn load_test_users(&mut self) -> Result<()> {
        self.auth_state.loading = true;
        self.auth_state.error = None;

        match self.api_client.get_test_users().await {
            Ok(users) => {
                if users.is_empty() {
                    self.auth_state.error = Some(
                        "No test users available. Please check server configuration.".to_string(),
                    );
                } else {
                    self.auth_state.test_users = users;
                }
                self.auth_state.loading = false;
            }
            Err(e) => {
                self.auth_state.error = Some(format!(
                    "Connection Error: Cannot reach server. Is it running? ({})",
                    e
                ));
                self.auth_state.loading = false;
            }
        }

        Ok(())
    }

    /// Login with selected user
    pub async fn login_selected_user(&mut self) -> Result<()> {
        if self.auth_state.test_users.is_empty() {
            return Ok(());
        }

        let selected_user = &self.auth_state.test_users[self.auth_state.selected_index];
        self.auth_state.loading = true;
        self.auth_state.error = None;

        match self.api_client.login(selected_user.username.clone()).await {
            Ok(response) => {
                self.auth_state.current_user = Some(response.user.clone());
                self.auth_state.loading = false;
                self.current_screen = Screen::Main;

                // Save session data
                let session_data = crate::config::SessionData {
                    username: response.user.username.clone(),
                    session_token: response.session_token.clone(),
                    user_id: response.user.id.to_string(),
                };

                if let Err(e) = self
                    .config_manager
                    .save_session(&self.instance_id, &session_data)
                {
                    eprintln!("Warning: Failed to save session: {}", e);
                }

                // Load user settings first (so posts use correct preferences)
                let _ = self.load_settings().await;

                // Load filter preference
                self.load_filter_preference();

                // Load posts after successful login (will use loaded settings and filter)
                let _ = self.load_posts().await;
            }
            Err(e) => {
                self.auth_state.error = Some(format!("Login failed: {}", e));
                self.auth_state.loading = false;
            }
        }

        Ok(())
    }

    /// Load posts from API
    pub async fn load_posts(&mut self) -> Result<()> {
        self.posts_state.loading = true;
        self.posts_state.error = None;

        // Yield to allow UI to render the loading state
        tokio::task::yield_now().await;

        // Add 200ms delay to ensure loading spinner is visible
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Get sort order and max posts from config
        let sort_order = self
            .settings_state
            .config
            .as_ref()
            .map(|c| c.sort_order.as_str().to_string())
            .unwrap_or_else(|| "newest".to_string());

        let max_posts = self
            .settings_state
            .config
            .as_ref()
            .map(|c| c.max_posts_display)
            .unwrap_or(25);

        // Apply current filter
        let result = match &self.posts_state.current_filter {
            PostFilter::All => {
                self.api_client
                    .get_posts(Some(max_posts), Some(sort_order.clone()), None, None)
                    .await
            }
            PostFilter::Hashtag(tag) => {
                self.api_client
                    .get_posts(
                        Some(max_posts),
                        Some(sort_order.clone()),
                        Some(tag.clone()),
                        None,
                    )
                    .await
            }
            PostFilter::User(user) => {
                self.api_client
                    .get_posts(
                        Some(max_posts),
                        Some(sort_order.clone()),
                        None,
                        Some(user.clone()),
                    )
                    .await
            }
            PostFilter::Multi { hashtags, users } => {
                // Fetch posts for each filter and combine them
                let mut all_posts = Vec::new();

                // Fetch for each hashtag
                for hashtag in hashtags {
                    if let Ok(posts) = self
                        .api_client
                        .get_posts(
                            Some(max_posts),
                            Some(sort_order.clone()),
                            Some(hashtag.clone()),
                            None,
                        )
                        .await
                    {
                        all_posts.extend(posts);
                    }
                }

                // Fetch for each user
                for username in users {
                    if let Ok(posts) = self
                        .api_client
                        .get_posts(
                            Some(max_posts),
                            Some(sort_order.clone()),
                            None,
                            Some(username.clone()),
                        )
                        .await
                    {
                        all_posts.extend(posts);
                    }
                }

                // Remove duplicates by post ID
                all_posts.sort_by(|a, b| b.created_at.cmp(&a.created_at)); // Sort by newest first
                all_posts.dedup_by(|a, b| a.id == b.id);

                // Limit to max_posts
                all_posts.truncate(max_posts as usize);

                Ok(all_posts)
            }
        };

        match result {
            Ok(posts) => {
                let has_posts = !posts.is_empty();
                self.posts_state.posts = posts;
                // Server now includes user_vote in each post
                if has_posts {
                    self.posts_state.list_state.select(Some(0));
                } else {
                    self.posts_state.list_state.select(None);
                }
                self.posts_state.loading = false;
            }
            Err(e) => {
                let error_msg = categorize_error(&e.to_string());
                self.posts_state.error = Some(error_msg);
                self.posts_state.loading = false;
            }
        }

        Ok(())
    }

    /// Vote on the currently selected post
    pub async fn vote_on_selected_post(&mut self, direction: &str) -> Result<()> {
        if let Some(selected_index) = self.posts_state.list_state.selected() {
            // Clear any previous errors
            self.posts_state.error = None;

            let selected_post = &mut self.posts_state.posts[selected_index];
            let post_id = selected_post.id;

            // Check if user has already voted on this post
            let previous_vote = selected_post.user_vote.clone();

            // If user is trying to vote the same direction again, silently ignore it
            if let Some(ref prev_direction) = previous_vote {
                if prev_direction == direction {
                    return Ok(());
                }
            }

            // Store original state for rollback
            let original_upvotes = selected_post.upvotes;
            let original_downvotes = selected_post.downvotes;
            let original_user_vote = selected_post.user_vote.clone();

            // Optimistic update: modify local state based on vote change
            match (&previous_vote, direction) {
                (None, "up") => {
                    // New upvote
                    selected_post.upvotes += 1;
                    selected_post.user_vote = Some("up".to_string());
                }
                (None, "down") => {
                    // New downvote
                    selected_post.downvotes += 1;
                    selected_post.user_vote = Some("down".to_string());
                }
                (Some(prev), "up") if prev == "down" => {
                    // Changing from downvote to upvote
                    selected_post.downvotes -= 1;
                    selected_post.upvotes += 1;
                    selected_post.user_vote = Some("up".to_string());
                }
                (Some(prev), "down") if prev == "up" => {
                    // Changing from upvote to downvote
                    selected_post.upvotes -= 1;
                    selected_post.downvotes += 1;
                    selected_post.user_vote = Some("down".to_string());
                }
                _ => {}
            }

            // Send vote to server (don't reload feed)
            match self
                .api_client
                .vote_on_post(post_id, direction.to_string())
                .await
            {
                Ok(_) => {
                    // Success - optimistic update is already applied
                    // Preserve selection - no reload, no re-sort
                }
                Err(e) => {
                    // Revert optimistic update on error
                    let selected_post = &mut self.posts_state.posts[selected_index];
                    selected_post.upvotes = original_upvotes;
                    selected_post.downvotes = original_downvotes;
                    selected_post.user_vote = original_user_vote;

                    // Categorize errors for better user feedback
                    let error_msg = categorize_error(&e.to_string());
                    self.posts_state.error = Some(error_msg);
                }
            }
        }
        Ok(())
    }

    /// Open the new post modal
    pub fn open_new_post_modal(&mut self) {
        self.posts_state.show_new_post_modal = true;
        self.posts_state.new_post_content.clear();
        self.input_mode = InputMode::Typing;
    }

    /// Close the new post modal
    pub fn close_new_post_modal(&mut self) {
        self.posts_state.show_new_post_modal = false;
        self.posts_state.new_post_content.clear();
        self.input_mode = InputMode::Navigation;
    }

    /// Open filter modal
    pub fn open_filter_modal(&mut self) {
        self.posts_state.show_filter_modal = true;
        self.posts_state.filter_modal_state.selected_index = 0;
        self.posts_state.filter_modal_state.search_input.clear();
        self.input_mode = InputMode::Navigation;

        // Reset and populate checked items from current active filter
        self.posts_state.filter_modal_state.checked_hashtags.clear();
        self.posts_state.filter_modal_state.checked_users.clear();

        if let PostFilter::Multi { hashtags, users } = &self.posts_state.current_filter {
            self.posts_state.filter_modal_state.checked_hashtags = hashtags.clone();
            self.posts_state.filter_modal_state.checked_users = users.clone();
        }

        // Lists will be loaded async in main loop
    }

    /// Load filter modal data (hashtags and following users)
    pub async fn load_filter_modal_data(&mut self) -> Result<()> {
        // Load followed hashtags
        match self.api_client.get_followed_hashtags().await {
            Ok(hashtags) => {
                self.posts_state.filter_modal_state.hashtag_list = hashtags;
            }
            Err(_) => {
                // Silently fail, just show empty list
                self.posts_state.filter_modal_state.hashtag_list.clear();
            }
        }

        // Load following users (people you follow)
        match self.api_client.get_following_list().await {
            Ok(following) => {
                self.posts_state.filter_modal_state.user_list =
                    following.into_iter().map(|user| user.username).collect();
            }
            Err(_) => {
                // Silently fail, just show empty list
                self.posts_state.filter_modal_state.user_list.clear();
            }
        }

        Ok(())
    }

    /// Close filter modal (keeps checked items for next time)
    pub fn close_filter_modal(&mut self) {
        self.posts_state.show_filter_modal = false;
        self.posts_state.filter_modal_state.search_mode = false;
        self.posts_state.filter_modal_state.search_input.clear();
        self.posts_state.filter_modal_state.search_results.clear();
        self.input_mode = InputMode::Navigation;
    }

    /// Cancel filter modal (clears checked items)
    pub fn cancel_filter_modal(&mut self) {
        self.posts_state.filter_modal_state.checked_hashtags.clear();
        self.posts_state.filter_modal_state.checked_users.clear();
        self.close_filter_modal();
    }

    /// Toggle selected item in filter modal (spacebar)
    pub fn toggle_filter_item(&mut self) {
        let selected_index = self.posts_state.filter_modal_state.selected_index;

        match self.posts_state.filter_modal_state.selected_tab {
            FilterTab::Hashtags => {
                if let Some(hashtag) = self
                    .posts_state
                    .filter_modal_state
                    .hashtag_list
                    .get(selected_index)
                {
                    let hashtag = hashtag.clone();
                    if let Some(pos) = self
                        .posts_state
                        .filter_modal_state
                        .checked_hashtags
                        .iter()
                        .position(|h| h == &hashtag)
                    {
                        // Already checked - uncheck it
                        self.posts_state
                            .filter_modal_state
                            .checked_hashtags
                            .remove(pos);
                    } else {
                        // Not checked - check it
                        self.posts_state
                            .filter_modal_state
                            .checked_hashtags
                            .push(hashtag);
                    }
                }
            }
            FilterTab::Users => {
                if let Some(username) = self
                    .posts_state
                    .filter_modal_state
                    .user_list
                    .get(selected_index)
                {
                    let username = username.clone();
                    if let Some(pos) = self
                        .posts_state
                        .filter_modal_state
                        .checked_users
                        .iter()
                        .position(|u| u == &username)
                    {
                        // Already checked - uncheck it
                        self.posts_state
                            .filter_modal_state
                            .checked_users
                            .remove(pos);
                    } else {
                        // Not checked - check it
                        self.posts_state
                            .filter_modal_state
                            .checked_users
                            .push(username);
                    }
                }
            }
            FilterTab::All => {
                // No toggle for "All" tab
            }
        }
    }

    /// Enter search mode in filter modal
    pub fn enter_search_mode(&mut self) {
        self.posts_state.filter_modal_state.search_mode = true;
        self.posts_state.filter_modal_state.search_input.clear();
        self.posts_state.filter_modal_state.search_results.clear();
        self.input_mode = InputMode::Typing;
    }

    /// Exit search mode in filter modal
    pub fn exit_search_mode(&mut self) {
        self.posts_state.filter_modal_state.search_mode = false;
        self.posts_state.filter_modal_state.search_input.clear();
        self.posts_state.filter_modal_state.search_results.clear();
        self.input_mode = InputMode::Navigation;
    }

    /// Search hashtags
    pub async fn search_hashtags(&mut self) -> Result<()> {
        let mut query = self.posts_state.filter_modal_state.search_input.clone();

        // Strip leading # if present (hashtags are stored without #)
        if query.starts_with('#') {
            query = query[1..].to_string();
        }

        if query.is_empty() {
            self.posts_state.filter_modal_state.search_results.clear();
            return Ok(());
        }

        // First try API search
        match self.api_client.search_hashtags(query.clone()).await {
            Ok(results) if !results.is_empty() => {
                self.posts_state.filter_modal_state.search_results = results;
                self.posts_state.filter_modal_state.selected_index = 0;
            }
            _ => {
                // Fallback: search in currently loaded posts
                let mut found_hashtags = std::collections::HashSet::new();
                let query_lower = query.to_lowercase();

                for post in &self.posts_state.posts {
                    for hashtag in &post.hashtags {
                        if hashtag.to_lowercase().contains(&query_lower) {
                            found_hashtags.insert(hashtag.clone());
                        }
                    }
                }

                let mut results: Vec<String> = found_hashtags.into_iter().collect();
                results.sort();

                self.posts_state.filter_modal_state.search_results = results;
                self.posts_state.filter_modal_state.selected_index = 0;
            }
        }
        Ok(())
    }

    /// Follow selected hashtag from search results
    pub async fn follow_selected_hashtag(&mut self) -> Result<()> {
        let selected_index = self.posts_state.filter_modal_state.selected_index;

        // Check if there are search results
        if self
            .posts_state
            .filter_modal_state
            .search_results
            .is_empty()
        {
            self.posts_state.error =
                Some("No search results. Try searching for a hashtag first.".to_string());
            return Ok(());
        }

        if let Some(hashtag) = self
            .posts_state
            .filter_modal_state
            .search_results
            .get(selected_index)
        {
            let hashtag_name = hashtag.clone();
            match self.api_client.follow_hashtag(hashtag_name.clone()).await {
                Ok(_) => {
                    // Reload followed hashtags from server
                    match self.api_client.get_followed_hashtags().await {
                        Ok(hashtags) => {
                            self.posts_state.filter_modal_state.hashtag_list = hashtags;
                        }
                        Err(_) => {
                            // Fallback: add to local list if reload fails
                            if !self
                                .posts_state
                                .filter_modal_state
                                .hashtag_list
                                .contains(&hashtag_name)
                            {
                                self.posts_state
                                    .filter_modal_state
                                    .hashtag_list
                                    .push(hashtag_name.clone());
                            }
                        }
                    }
                    // Clear any errors
                    self.posts_state.error = None;
                    // Exit search mode and reset to hashtags tab
                    self.exit_search_mode();
                    self.posts_state.filter_modal_state.selected_index = 0;
                }
                Err(e) => {
                    self.posts_state.error = Some(format!("Failed to follow hashtag: {}", e));
                }
            }
        }
        Ok(())
    }

    /// Toggle follow/unfollow for selected hashtag in followed list
    pub async fn toggle_follow_hashtag(&mut self) -> Result<()> {
        let selected_index = self.posts_state.filter_modal_state.selected_index;

        // Calculate offset for search results
        let search_count = if !self.posts_state.filter_modal_state.search_input.is_empty() {
            self.posts_state.filter_modal_state.search_results.len()
        } else {
            0
        };

        // Only toggle if we're in the followed list, not search results
        if selected_index < search_count {
            return Ok(());
        }

        let list_index = selected_index - search_count;
        if let Some(hashtag) = self
            .posts_state
            .filter_modal_state
            .hashtag_list
            .get(list_index)
        {
            let hashtag_name = hashtag.clone();

            // Unfollow the hashtag (since it's in the followed list)
            match self.api_client.unfollow_hashtag(hashtag_name.clone()).await {
                Ok(_) => {
                    // Remove from local list immediately for responsive UI
                    self.posts_state
                        .filter_modal_state
                        .hashtag_list
                        .remove(list_index);

                    // Also remove from checked list if it was checked
                    if let Some(pos) = self
                        .posts_state
                        .filter_modal_state
                        .checked_hashtags
                        .iter()
                        .position(|h| h == &hashtag_name)
                    {
                        self.posts_state
                            .filter_modal_state
                            .checked_hashtags
                            .remove(pos);
                    }

                    // Adjust selection if needed
                    if self.posts_state.filter_modal_state.selected_index > 0
                        && self.posts_state.filter_modal_state.selected_index
                            >= search_count + self.posts_state.filter_modal_state.hashtag_list.len()
                    {
                        self.posts_state.filter_modal_state.selected_index -= 1;
                    }

                    // Clear any errors
                    self.posts_state.error = None;
                }
                Err(e) => {
                    self.posts_state.error = Some(format!("Failed to unfollow hashtag: {}", e));
                }
            }
        }
        Ok(())
    }

    /// Apply filter and reload posts
    pub async fn apply_filter(&mut self, filter: PostFilter) -> Result<()> {
        self.posts_state.current_filter = filter.clone();
        self.close_filter_modal();

        // Save filter preference
        self.save_filter_preference();

        // Set flag to trigger load in main loop instead of blocking here
        self.posts_state.pending_load = true;
        Ok(())
    }

    /// Save current filter preference to disk
    fn save_filter_preference(&self) {
        if let Some(user) = &self.auth_state.current_user {
            let prefs = self.posts_state.current_filter.to_preferences();
            let _ = self
                .config_manager
                .save_preferences(&user.id.to_string(), &prefs);
        }
    }

    /// Load filter preference from disk
    pub fn load_filter_preference(&mut self) {
        if let Some(user) = &self.auth_state.current_user {
            if let Ok(Some(prefs)) = self.config_manager.load_preferences(&user.id.to_string()) {
                self.posts_state.current_filter = PostFilter::from_preferences(&prefs);
            }
        }
    }

    /// Add character to new post content
    pub fn add_char_to_post(&mut self, c: char) {
        if self.posts_state.new_post_content.len() < 280 {
            self.posts_state.new_post_content.push(c);
        }
    }

    /// Remove last character from new post content
    pub fn remove_char_from_post(&mut self) {
        self.posts_state.new_post_content.pop();
    }

    /// Submit new post
    pub async fn submit_new_post(&mut self) -> Result<()> {
        let trimmed = self.posts_state.new_post_content.trim();

        // Validate empty input
        if trimmed.is_empty() {
            self.posts_state.error = Some(
                "Validation Error: Cannot post empty content. Type something first!".to_string(),
            );
            return Ok(());
        }

        // Validate character limit
        let char_count = crate::emoji::count_characters(&self.posts_state.new_post_content);
        if char_count > 280 {
            self.posts_state.error = Some(format!(
                "Validation Error: Post exceeds 280 characters (current: {})",
                char_count
            ));
            return Ok(());
        }

        // Clear any previous errors
        self.posts_state.error = None;

        // Parse emoji shortcodes before sending
        let content = crate::emoji::parse_emoji_shortcodes(&self.posts_state.new_post_content);

        match self.api_client.create_post(content).await {
            Ok(_) => {
                // Close modal and refresh posts (also switches to navigation mode)
                self.close_new_post_modal();
                self.load_posts().await?;
            }
            Err(e) => {
                // Categorize errors for better user feedback
                let error_msg = categorize_error(&e.to_string());
                self.posts_state.error = Some(error_msg);
            }
        }

        Ok(())
    }

    /// Load profile data
    pub async fn load_profile(&mut self) -> Result<()> {
        if let Some(user) = &self.auth_state.current_user {
            self.profile_state.loading = true;
            self.profile_state.error = None;

            // Load profile
            match self.api_client.get_profile(user.id).await {
                Ok(profile) => {
                    self.profile_state.profile = Some(profile);
                }
                Err(e) => {
                    let error_msg = categorize_error(&e.to_string());
                    self.profile_state.error =
                        Some(format!("{} (Switch tabs to retry)", error_msg));
                    self.profile_state.loading = false;
                    return Ok(());
                }
            }

            // Load user's posts
            match self
                .api_client
                .get_posts(Some(100), Some("newest".to_string()), None, None)
                .await
            {
                Ok(posts) => {
                    // Filter to only show current user's posts
                    self.profile_state.user_posts = posts
                        .into_iter()
                        .filter(|p| p.author_id == user.id)
                        .collect();
                    if !self.profile_state.user_posts.is_empty() {
                        self.profile_state.list_state.select(Some(0));
                    } else {
                        self.profile_state.list_state.select(None);
                    }
                    self.profile_state.loading = false;
                }
                Err(e) => {
                    let error_msg = categorize_error(&e.to_string());
                    self.profile_state.error =
                        Some(format!("{} (Switch tabs to retry)", error_msg));
                    self.profile_state.loading = false;
                }
            }
        }

        Ok(())
    }

    /// Open edit bio modal
    pub fn open_edit_bio_modal(&mut self) {
        if let Some(profile) = &self.profile_state.profile {
            self.profile_state.show_edit_bio_modal = true;
            self.profile_state.edit_bio_content = profile.bio.clone().unwrap_or_default();
            // Set cursor to end (character count, not byte length)
            self.profile_state.edit_bio_cursor_position =
                self.profile_state.edit_bio_content.chars().count();
            self.input_mode = InputMode::Typing;
        }
    }

    /// Close edit bio modal
    pub fn close_edit_bio_modal(&mut self) {
        self.profile_state.show_edit_bio_modal = false;
        self.profile_state.edit_bio_content.clear();
        self.profile_state.edit_bio_cursor_position = 0;
        self.input_mode = InputMode::Navigation;
    }

    /// Add character to bio
    pub fn add_char_to_bio(&mut self, c: char) {
        // Check character count, not byte length
        let char_count = self.profile_state.edit_bio_content.chars().count();
        if char_count < 160 {
            // Find the byte position for the cursor
            let byte_pos = self
                .profile_state
                .edit_bio_content
                .char_indices()
                .nth(self.profile_state.edit_bio_cursor_position)
                .map(|(pos, _)| pos)
                .unwrap_or(self.profile_state.edit_bio_content.len());

            self.profile_state.edit_bio_content.insert(byte_pos, c);
            self.profile_state.edit_bio_cursor_position += 1;
        }
    }

    /// Remove character before cursor (backspace)
    pub fn remove_char_from_bio(&mut self) {
        if self.profile_state.edit_bio_cursor_position > 0 {
            // Find the byte position of the character to remove
            if let Some((byte_pos, _)) = self
                .profile_state
                .edit_bio_content
                .char_indices()
                .nth(self.profile_state.edit_bio_cursor_position - 1)
            {
                self.profile_state.edit_bio_content.remove(byte_pos);
                self.profile_state.edit_bio_cursor_position -= 1;
            }
        }
    }

    /// Submit bio update
    pub async fn submit_bio_update(&mut self) -> Result<()> {
        if let Some(user) = &self.auth_state.current_user {
            self.profile_state.error = None;

            let bio = self.profile_state.edit_bio_content.clone();

            match self.api_client.update_bio(user.id, bio).await {
                Ok(_) => {
                    self.close_edit_bio_modal();
                    self.load_profile().await?;
                }
                Err(e) => {
                    // Parse error message for specific cases
                    let error_msg = e.to_string();
                    let parsed_error = if error_msg.contains("401") || error_msg.contains("403") {
                        "Authorization Error: You can only edit your own profile".to_string()
                    } else if error_msg.contains("400") {
                        format!("Validation Error: {}", error_msg)
                    } else if error_msg.contains("connection") || error_msg.contains("timeout") {
                        "Network Error: Connection failed - check your network and try again"
                            .to_string()
                    } else {
                        format!("Failed to update bio: {}", error_msg)
                    };
                    self.profile_state.error = Some(parsed_error);
                }
            }
        }

        Ok(())
    }

    // ============================================================================
    // ============================================================================
    // FRIENDS METHODS
    // ============================================================================

    /// Load social connections (following, followers, mutual friends)
    pub async fn load_social_connections(&mut self) -> Result<()> {
        self.friends_state.loading = true;
        self.friends_state.error = None;

        // Load all three lists
        let following_result = self.api_client.get_following_list().await;
        let followers_result = self.api_client.get_followers_list().await;
        let mutual_result = self.api_client.get_mutual_friends_list().await;

        // Check for errors and provide detailed messages
        if let Err(e) = &following_result {
            let error_msg = format!("Failed to load following: {}", e);
            self.friends_state.error = Some(error_msg.clone());
            self.friends_state.loading = false;
            return Err(anyhow::anyhow!(error_msg));
        }
        if let Err(e) = &followers_result {
            let error_msg = format!("Failed to load followers: {}", e);
            self.friends_state.error = Some(error_msg.clone());
            self.friends_state.loading = false;
            return Err(anyhow::anyhow!(error_msg));
        }
        if let Err(e) = &mutual_result {
            let error_msg = format!("Failed to load mutual friends: {}", e);
            self.friends_state.error = Some(error_msg.clone());
            self.friends_state.loading = false;
            return Err(anyhow::anyhow!(error_msg));
        }

        // All succeeded, unwrap safely
        let following = following_result.unwrap();
        let followers = followers_result.unwrap();
        let mutual = mutual_result.unwrap();

        self.friends_state.following = following
            .into_iter()
            .map(|u| UserInfo {
                id: u.id,
                username: u.username,
                follower_count: u.follower_count,
                following_count: u.following_count,
            })
            .collect();

        self.friends_state.followers = followers
            .into_iter()
            .map(|u| UserInfo {
                id: u.id,
                username: u.username,
                follower_count: u.follower_count,
                following_count: u.following_count,
            })
            .collect();

        self.friends_state.mutual_friends = mutual
            .into_iter()
            .map(|u| UserInfo {
                id: u.id,
                username: u.username,
                follower_count: u.follower_count,
                following_count: u.following_count,
            })
            .collect();

        self.friends_state.loading = false;
        Ok(())
    }

    /// Close social connections modal
    pub fn close_friends_modal(&mut self) {
        self.friends_state.show_friends_modal = false;
        self.friends_state.search_mode = false;
        self.friends_state.search_query.clear();
        self.friends_state.selected_index = 0;
        self.friends_state.error = None;
    }

    /// Get filtered user list based on current tab and search
    pub fn get_filtered_social_list(&self) -> Vec<&UserInfo> {
        let list = match self.friends_state.selected_tab {
            SocialTab::Following => &self.friends_state.following,
            SocialTab::Followers => &self.friends_state.followers,
            SocialTab::MutualFriends => &self.friends_state.mutual_friends,
        };

        if self.friends_state.search_query.is_empty() {
            list.iter().collect()
        } else {
            let query = self.friends_state.search_query.to_lowercase();
            list.iter()
                .filter(|u| u.username.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Handle social connections modal key events
    pub fn handle_friends_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        // If in search mode, handle search input
        if self.friends_state.search_mode {
            match key.code {
                KeyCode::Char(c) => {
                    self.friends_state.search_query.push(c);
                    self.friends_state.selected_index = 0; // Reset selection when searching
                }
                KeyCode::Backspace => {
                    self.friends_state.search_query.pop();
                    self.friends_state.selected_index = 0;
                }
                KeyCode::Esc => {
                    self.friends_state.search_mode = false;
                    self.friends_state.search_query.clear();
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.friends_state.selected_index > 0 {
                    self.friends_state.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                let max_index = self.get_filtered_social_list().len().saturating_sub(1);
                if self.friends_state.selected_index < max_index {
                    self.friends_state.selected_index += 1;
                }
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                // Cycle through tabs
                self.friends_state.selected_tab = match self.friends_state.selected_tab {
                    SocialTab::Following => SocialTab::Followers,
                    SocialTab::Followers => SocialTab::MutualFriends,
                    SocialTab::MutualFriends => SocialTab::Following,
                };
                self.friends_state.selected_index = 0;
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                // Cycle through tabs backwards
                self.friends_state.selected_tab = match self.friends_state.selected_tab {
                    SocialTab::Following => SocialTab::MutualFriends,
                    SocialTab::Followers => SocialTab::Following,
                    SocialTab::MutualFriends => SocialTab::Followers,
                };
                self.friends_state.selected_index = 0;
            }
            KeyCode::Char('/') => {
                // Enter search mode
                self.friends_state.search_mode = true;
                self.friends_state.search_query.clear();
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // View selected user's profile (handled in main.rs)
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Follow/unfollow selected user (handled in main.rs)
            }
            _ => {}
        }
        Ok(())
    }

    /// Close DM error modal
    pub fn close_dm_error_modal(&mut self) {
        self.dms_state.show_dm_error_modal = false;
        self.dms_state.dm_error_message.clear();
        self.dms_state.failed_username = None;
    }

    /// Clear DM error message (non-modal error text)
    pub fn clear_dm_error(&mut self) {
        self.dms_state.error = None;
    }

    /// Handle DM error modal key events
    pub fn handle_dm_error_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Enter {
            // Just close the error modal - user can search for the person in social modal
            self.close_dm_error_modal();
        }
        Ok(())
    }

    // ===== Hashtag Management Functions =====

    /// Load hashtags list from API
    pub async fn load_hashtags(&mut self) -> Result<()> {
        self.hashtags_state.loading = true;
        self.hashtags_state.error = None;

        match self.api_client.get_followed_hashtags().await {
            Ok(hashtags) => {
                self.hashtags_state.hashtags = hashtags;
                self.hashtags_state.loading = false;
                Ok(())
            }
            Err(e) => {
                self.hashtags_state.error = Some(format!("Failed to load hashtags: {}", e));
                self.hashtags_state.loading = false;
                Err(e.into())
            }
        }
    }

    /// Follow a hashtag by name
    pub async fn follow_hashtag(&mut self, name: &str) -> Result<()> {
        self.hashtags_state.error = None;

        // Strip leading # if present
        let clean_name = name.strip_prefix('#').unwrap_or(name);

        match self.api_client.follow_hashtag(clean_name.to_string()).await {
            Ok(_) => {
                // Reload hashtags list
                self.load_hashtags().await?;
                // Also reload filter modal data to update the filter list
                self.load_filter_modal_data().await?;
                self.hashtags_state.add_hashtag_name.clear();
                self.hashtags_state.show_add_hashtag_input = false;
                Ok(())
            }
            Err(e) => {
                self.hashtags_state.error = Some(format!("Failed to follow #{}", clean_name));
                Err(e.into())
            }
        }
    }

    /// Unfollow a hashtag by name
    pub async fn unfollow_hashtag(&mut self, name: &str) -> Result<()> {
        self.hashtags_state.error = None;

        match self.api_client.unfollow_hashtag(name.to_string()).await {
            Ok(_) => {
                // Reload hashtags list
                self.load_hashtags().await?;
                // Also reload filter modal data to update the filter list
                self.load_filter_modal_data().await?;
                Ok(())
            }
            Err(e) => {
                self.hashtags_state.error = Some(format!("Failed to unfollow hashtag: {}", e));
                Err(e.into())
            }
        }
    }

    /// Close hashtags modal
    pub fn close_hashtags_modal(&mut self) {
        self.hashtags_state.show_hashtags_modal = false;
        self.hashtags_state.show_add_hashtag_input = false;
        self.hashtags_state.add_hashtag_name.clear();
        self.hashtags_state.error = None;
    }

    /// Handle hashtags modal key events
    pub fn handle_hashtags_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        // If in unfollow confirmation mode, handle that first
        if self.hashtags_state.show_unfollow_confirmation {
            return self.handle_unfollow_confirmation_keys(key);
        }

        // If in add hashtag input mode, handle that separately
        if self.hashtags_state.show_add_hashtag_input {
            return self.handle_add_hashtag_input_keys(key);
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.hashtags_state.selected_hashtag > 0 {
                    self.hashtags_state.selected_hashtag -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                // Max index is hashtags.len() (includes "Follow Hashtag" option)
                let max_index = self.hashtags_state.hashtags.len();
                if self.hashtags_state.selected_hashtag < max_index {
                    self.hashtags_state.selected_hashtag += 1;
                }
            }
            KeyCode::Enter => {
                // If selected "Follow Hashtag" option (last item)
                if self.hashtags_state.selected_hashtag == self.hashtags_state.hashtags.len() {
                    self.hashtags_state.show_add_hashtag_input = true;
                    self.hashtags_state.add_hashtag_name.clear();
                    self.hashtags_state.error = None;
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                // Unfollow selected hashtag (if not on "Follow Hashtag" option)
                if self.hashtags_state.selected_hashtag < self.hashtags_state.hashtags.len() {
                    let hashtag =
                        self.hashtags_state.hashtags[self.hashtags_state.selected_hashtag].clone();
                    self.hashtags_state.show_unfollow_confirmation = true;
                    self.hashtags_state.hashtag_to_unfollow = Some(hashtag);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle add hashtag input key events
    pub fn handle_add_hashtag_input_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                // Submit hashtag (will be handled async in main.rs)
                // Don't close input here, let main.rs handle it after API call
            }
            KeyCode::Esc => {
                // Cancel input
                self.hashtags_state.show_add_hashtag_input = false;
                self.hashtags_state.add_hashtag_name.clear();
                self.hashtags_state.error = None;
            }
            KeyCode::Char(c) => {
                self.hashtags_state.add_hashtag_name.push(c);
            }
            KeyCode::Backspace => {
                self.hashtags_state.add_hashtag_name.pop();
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle unfollow hashtag confirmation key events
    pub fn handle_unfollow_confirmation_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Enter and 'y' confirm unfollow (handled in main.rs async)
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Will be handled in main.rs
            }
            // Any other key cancels
            _ => {
                self.hashtags_state.show_unfollow_confirmation = false;
                self.hashtags_state.hashtag_to_unfollow = None;
            }
        }
        Ok(())
    }

    // UNIFIED COMPOSER METHODS (using tui-textarea)
    // ============================================================================

    /// Open composer for new post
    pub fn open_composer_new_post(&mut self) {
        self.composer_state.mode = Some(ComposerMode::NewPost);
        let mut textarea = TextArea::default();
        // Enable hard tab indent for better wrapping behavior
        textarea.set_hard_tab_indent(true);
        // Set styles immediately to avoid rendering glitches
        self.apply_composer_styling(&mut textarea);
        self.composer_state.textarea = textarea;
        self.composer_state.max_chars = 280;
        self.input_mode = InputMode::Typing;
    }

    /// Open composer for reply
    pub fn open_composer_reply(
        &mut self,
        parent_post_id: Uuid,
        parent_author: String,
        parent_content: String,
    ) {
        self.composer_state.mode = Some(ComposerMode::Reply {
            parent_post_id,
            parent_author,
            parent_content,
        });
        let mut textarea = TextArea::default();
        // Enable hard tab indent for better wrapping behavior
        textarea.set_hard_tab_indent(true);
        // Set styles immediately to avoid rendering glitches
        self.apply_composer_styling(&mut textarea);
        self.composer_state.textarea = textarea;
        self.composer_state.max_chars = 280;
        self.input_mode = InputMode::Typing;
    }

    /// Open composer for editing post
    pub fn open_composer_edit_post(&mut self, post_id: Uuid, current_content: String) {
        self.composer_state.mode = Some(ComposerMode::EditPost { post_id });
        let mut textarea = TextArea::from(current_content.lines());
        // Enable hard tab indent for better wrapping behavior
        textarea.set_hard_tab_indent(true);
        // Set styles immediately to avoid rendering glitches
        self.apply_composer_styling(&mut textarea);
        self.composer_state.textarea = textarea;
        self.composer_state.max_chars = 280;
        self.input_mode = InputMode::Typing;
    }

    /// Open composer for editing bio
    pub fn open_composer_edit_bio(&mut self, current_bio: String) {
        self.composer_state.mode = Some(ComposerMode::EditBio);
        let mut textarea = TextArea::from(current_bio.lines());
        // Enable hard tab indent for better wrapping behavior
        textarea.set_hard_tab_indent(true);
        // Set styles immediately to avoid rendering glitches
        self.apply_composer_styling(&mut textarea);
        self.composer_state.textarea = textarea;
        self.composer_state.max_chars = 160;
        self.input_mode = InputMode::Typing;
    }

    /// Close composer
    pub fn close_composer(&mut self) {
        self.composer_state.mode = None;
        let mut textarea = TextArea::default();
        textarea.set_hard_tab_indent(true);
        self.apply_composer_styling(&mut textarea);
        self.composer_state.textarea = textarea;
        self.input_mode = InputMode::Navigation;
    }

    /// Apply consistent styling to composer TextArea
    fn apply_composer_styling(&self, textarea: &mut TextArea) {
        use crate::ui::theme::get_theme_colors;
        let theme = get_theme_colors(self);
        
        textarea.set_style(
            Style::default()
                .fg(theme.primary)  // Use primary color for better visibility
        );
        textarea.set_cursor_style(
            Style::default()
                .fg(theme.background)
                .bg(theme.primary)  // Visible cursor
        );
        textarea.set_cursor_line_style(
            Style::default()  // No special cursor line styling
        );
    }

    /// Handle keyboard input for composer (delegates to TextArea)
    pub fn handle_composer_input(&mut self, key: KeyEvent) {
        // Check if this is a character input that would exceed the limit
        if let KeyCode::Char(_c) = key.code {
            // Check current character count
            let current_count = self.composer_state.char_count();

            // Only allow input if under the limit
            if current_count >= self.composer_state.max_chars {
                // Don't process this character - limit reached
                return;
            }
        }

        // Convert KeyEvent to tui_textarea::Input and process
        use tui_textarea::Input;
        let input = Input::from(crossterm::event::Event::Key(key));
        self.composer_state.textarea.input(input);

        // After input, check if we need to wrap the current line
        // This ensures text stays visible within the modal
        self.wrap_composer_text_if_needed();
    }

    /// Wrap text in composer if current line exceeds reasonable width
    fn wrap_composer_text_if_needed(&mut self) {
        crate::text_wrapper::wrap_textarea_if_needed(
            &mut self.composer_state.textarea,
            crate::text_wrapper::WrapConfig::COMPOSER,
        );
    }

    /// Submit composer content based on mode
    pub async fn submit_composer(&mut self) -> Result<()> {
        let content = self.composer_state.get_content();
        let trimmed = content.trim();

        // Validate empty input
        if trimmed.is_empty() {
            match &self.composer_state.mode {
                Some(ComposerMode::NewPost) => {
                    self.posts_state.error =
                        Some("Validation Error: Cannot post empty content.".to_string());
                }
                Some(ComposerMode::Reply { .. }) => {
                    if let Some(detail_state) = &mut self.post_detail_state {
                        detail_state.error =
                            Some("Validation Error: Cannot post empty reply.".to_string());
                    }
                }
                Some(ComposerMode::EditPost { .. }) => {
                    if let Some(detail_state) = &mut self.post_detail_state {
                        detail_state.error =
                            Some("Validation Error: Cannot save empty post.".to_string());
                    }
                }
                Some(ComposerMode::EditBio) => {
                    self.profile_state.error =
                        Some("Validation Error: Bio cannot be empty.".to_string());
                }
                None => {}
            }
            return Ok(());
        }

        // Validate character limit
        let char_count = self.composer_state.char_count();
        if char_count > self.composer_state.max_chars {
            let error_msg = format!(
                "Validation Error: Content exceeds {} characters (current: {})",
                self.composer_state.max_chars, char_count
            );
            match &self.composer_state.mode {
                Some(ComposerMode::NewPost) => {
                    self.posts_state.error = Some(error_msg);
                }
                Some(ComposerMode::Reply { .. }) | Some(ComposerMode::EditPost { .. }) => {
                    if let Some(detail_state) = &mut self.post_detail_state {
                        detail_state.error = Some(error_msg);
                    }
                }
                Some(ComposerMode::EditBio) => {
                    self.profile_state.error = Some(error_msg);
                }
                None => {}
            }
            return Ok(());
        }

        // Parse emoji shortcodes
        let parsed_content = crate::emoji::parse_emoji_shortcodes(&content);

        // Submit based on mode
        match &self.composer_state.mode {
            Some(ComposerMode::NewPost) => {
                self.posts_state.error = None;
                match self.api_client.create_post(parsed_content).await {
                    Ok(_) => {
                        self.close_composer();
                        self.load_posts().await?;
                    }
                    Err(e) => {
                        self.posts_state.error = Some(categorize_error(&e.to_string()));
                    }
                }
            }
            Some(ComposerMode::Reply { parent_post_id, .. }) => {
                let post_id = *parent_post_id;
                
                // Get the root post ID from the modal (the thread we're viewing)
                let root_post_id = self.post_detail_state
                    .as_ref()
                    .and_then(|s| s.post.as_ref().map(|p| p.id))
                    .unwrap_or(post_id);
                
                // Debug logging to file
                use std::io::Write;
                let mut log = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("fido_debug.log")
                    .ok();
                
                if let Some(ref mut f) = log {
                    let _ = writeln!(f, "\n=== REPLY SUBMISSION START ===");
                    let _ = writeln!(f, "Before reply - viewing_post_detail={}, show_full_post_modal={}", 
                        self.viewing_post_detail,
                        self.post_detail_state.as_ref().map(|s| s.show_full_post_modal).unwrap_or(false));
                }
                
                if let Some(detail_state) = &mut self.post_detail_state {
                    detail_state.error = None;
                }
                match self.api_client.create_reply(post_id, parsed_content).await {
                    Ok(new_reply) => {
                        let new_reply_id = new_reply.id;
                        
                        if let Some(ref mut f) = log {
                            let _ = writeln!(f, "Reply created successfully, new_reply_id={}", new_reply_id);
                        }
                        
                        // Optimistic update: increment reply count in cached post
                        if let Some(cached_post) =
                            self.posts_state.posts.iter_mut().find(|p| p.id == post_id)
                        {
                            cached_post.reply_count += 1;
                        }

                        self.close_composer();
                        
                        if let Some(ref mut f) = log {
                            let _ = writeln!(f, "After close_composer - viewing_post_detail={}", self.viewing_post_detail);
                        }
                        
                        // Ensure we stay in thread view
                        self.viewing_post_detail = true;
                        
                        if let Some(ref mut f) = log {
                            let _ = writeln!(f, "Before load_post_detail - root_post_id={}", root_post_id);
                        }
                        
                        // Reload the root thread, not the parent post
                        self.load_post_detail(root_post_id).await?;
                        
                        if let Some(ref mut f) = log {
                            let _ = writeln!(f, "After load_post_detail - viewing_post_detail={}, show_full_post_modal={}, post_detail_state.is_some()={}", 
                                self.viewing_post_detail,
                                self.post_detail_state.as_ref().map(|s| s.show_full_post_modal).unwrap_or(false),
                                self.post_detail_state.is_some());
                        }
                        
                        // Explicitly ensure modal is open after reload
                        if let Some(detail_state) = &mut self.post_detail_state {
                            detail_state.show_full_post_modal = true;
                            detail_state.full_post_modal_id = Some(root_post_id);
                            if let Some(ref mut f) = log {
                                let _ = writeln!(f, "Explicitly set show_full_post_modal=true");
                            }
                        }
                        
                        // Select the newly created reply in the modal
                        self.select_reply_in_modal(new_reply_id);
                        
                        if let Some(ref mut f) = log {
                            let _ = writeln!(f, "Final state - viewing_post_detail={}, show_full_post_modal={}", 
                                self.viewing_post_detail,
                                self.post_detail_state.as_ref().map(|s| s.show_full_post_modal).unwrap_or(false));
                            let _ = writeln!(f, "=== REPLY SUBMISSION END ===\n");
                        }
                    }
                    Err(e) => {
                        if let Some(detail_state) = &mut self.post_detail_state {
                            detail_state.error = Some(categorize_error(&e.to_string()));
                        }
                    }
                }
            }
            Some(ComposerMode::EditPost { post_id }) => {
                let post_id = *post_id;
                if let Some(detail_state) = &mut self.post_detail_state {
                    detail_state.error = None;
                }
                match self.api_client.update_post(post_id, parsed_content).await {
                    Ok(_) => {
                        self.close_composer();
                        self.load_post_detail(post_id).await?;
                    }
                    Err(e) => {
                        if let Some(detail_state) = &mut self.post_detail_state {
                            detail_state.error = Some(categorize_error(&e.to_string()));
                        }
                    }
                }
            }
            Some(ComposerMode::EditBio) => {
                if let Some(user) = &self.auth_state.current_user {
                    self.profile_state.error = None;
                    match self.api_client.update_bio(user.id, parsed_content).await {
                        Ok(_) => {
                            self.close_composer();
                            self.load_profile().await?;
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            let parsed_error = if error_msg.contains("401")
                                || error_msg.contains("403")
                            {
                                "Authorization Error: You can only edit your own profile"
                                    .to_string()
                            } else if error_msg.contains("400") {
                                format!("Validation Error: {}", error_msg)
                            } else if error_msg.contains("connection")
                                || error_msg.contains("timeout")
                            {
                                "Network Error: Connection failed - check your network and try again".to_string()
                            } else {
                                format!("Failed to update bio: {}", error_msg)
                            };
                            self.profile_state.error = Some(parsed_error);
                        }
                    }
                }
            }
            None => {}
        }

        Ok(())
    }

    /// Load DM conversations
    pub async fn load_conversations(&mut self) -> Result<()> {
        self.dms_state.loading = true;
        self.dms_state.error = None;

        match self.api_client.get_conversations().await {
            Ok(convos) => {
                // Parse conversations from JSON
                self.dms_state.conversations = convos
                    .iter()
                    .filter_map(|c| {
                        Some(Conversation {
                            other_user_id: c.get("other_user_id")?.as_str()?.parse().ok()?,
                            other_username: c.get("other_username")?.as_str()?.to_string(),
                            last_message: c.get("last_message")?.as_str()?.to_string(),
                            last_message_time: c
                                .get("last_message_time")?
                                .as_str()?
                                .parse()
                                .ok()?,
                            unread_count: c.get("unread_count")?.as_i64()? as i32,
                        })
                    })
                    .collect();

                // Update unread_counts HashMap from conversations
                self.dms_state.unread_counts.clear();
                for convo in &self.dms_state.conversations {
                    self.dms_state
                        .unread_counts
                        .insert(convo.other_user_id, convo.unread_count as usize);
                }

                // Select first conversation if available
                if !self.dms_state.conversations.is_empty() {
                    self.dms_state.selected_conversation_index = Some(0);
                    self.dms_state.needs_message_load = true;
                }

                self.dms_state.loading = false;
            }
            Err(e) => {
                let error_msg = categorize_error(&e.to_string());
                self.dms_state.error = Some(error_msg);
                self.dms_state.loading = false;
            }
        }

        Ok(())
    }

    /// Load messages for selected conversation
    pub async fn load_conversation_messages(&mut self) -> Result<()> {
        if self.dms_state.conversations.is_empty() {
            return Ok(());
        }

        // Check if a conversation is selected (not the "New Conversation" button)
        let selected_index = match self.dms_state.selected_conversation_index {
            Some(usize::MAX) => return Ok(()), // "New Conversation" button selected, nothing to load
            Some(index) => index,
            None => return Ok(()), // No conversation selected, nothing to load
        };

        let conversation = &self.dms_state.conversations[selected_index];
        let other_user_id = conversation.other_user_id;

        match self.api_client.get_conversation(other_user_id).await {
            Ok(messages) => {
                self.dms_state.messages = messages;

                // Mark conversation as read when opening it
                self.mark_conversation_as_read(other_user_id).await?;
            }
            Err(e) => {
                let error_msg = categorize_error(&e.to_string());
                self.dms_state.error = Some(error_msg);
            }
        }

        Ok(())
    }

    /// Mark conversation as read
    pub async fn mark_conversation_as_read(&mut self, user_id: uuid::Uuid) -> Result<()> {
        // Set current conversation user
        self.dms_state.current_conversation_user = Some(user_id);

        // Call API to mark messages as read
        match self.api_client.mark_messages_read(user_id).await {
            Ok(_) => {
                // Clear unread count for this user
                self.dms_state.unread_counts.insert(user_id, 0);

                // Update local message state
                for msg in self.dms_state.messages.iter_mut() {
                    msg.is_read = true;
                }

                // Update conversation unread count
                if let Some(convo) = self
                    .dms_state
                    .conversations
                    .iter_mut()
                    .find(|c| c.other_user_id == user_id)
                {
                    convo.unread_count = 0;
                }
            }
            Err(e) => {
                // Log error but don't fail the operation
                eprintln!("Warning: Failed to mark messages as read: {}", e);
            }
        }

        Ok(())
    }

    /// Handle keyboard input for DM message textarea
    pub fn handle_dm_input(&mut self, key: KeyEvent) {
        // DMs don't have a strict character limit, but we can add one if needed
        // For now, just pass through to TextArea which handles wrapping
        use tui_textarea::Input;
        let input = Input::from(crossterm::event::Event::Key(key));
        self.dms_state.message_textarea.input(input);

        // After input, check if we need to wrap the current line
        self.wrap_dm_text_if_needed();
    }

    /// Wrap text in DM input if current line exceeds reasonable width
    fn wrap_dm_text_if_needed(&mut self) {
        crate::text_wrapper::wrap_textarea_if_needed(
            &mut self.dms_state.message_textarea,
            crate::text_wrapper::WrapConfig::DM_PANEL,
        );
    }

    /// Get DM message content from textarea
    pub fn get_dm_message_content(&self) -> String {
        self.dms_state.message_textarea.lines().join("\n")
    }

    /// Clear DM message textarea
    pub fn clear_dm_message(&mut self) {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_style(Style::default());
        // Enable hard tab indent for better wrapping behavior
        textarea.set_hard_tab_indent(true);
        self.dms_state.message_textarea = textarea;
    }

    /// Check if DM message is empty
    pub fn is_dm_message_empty(&self) -> bool {
        self.get_dm_message_content().trim().is_empty()
    }

    /// Send DM
    pub async fn send_dm(&mut self) -> Result<()> {
        let content = self.get_dm_message_content();
        let trimmed = content.trim();

        // Validate empty input
        if trimmed.is_empty() {
            self.dms_state.error = Some(
                "Validation Error: Cannot send empty message. Type something first!".to_string(),
            );
            return Ok(());
        }

        self.dms_state.error = None;

        // Check if this is a pending new conversation
        let to_username = if let Some(pending_username) =
            &self.dms_state.pending_conversation_username
        {
            pending_username.clone()
        } else {
            // Regular conversation - get from selected
            let selected_index = match self.dms_state.selected_conversation_index {
                Some(usize::MAX) => {
                    self.dms_state.error =
                        Some("Press Enter on 'New Conversation' to start a new chat.".to_string());
                    return Ok(());
                }
                Some(index) => index,
                None => {
                    self.dms_state.error = Some(
                        "No conversation selected. Use arrow keys to select a conversation."
                            .to_string(),
                    );
                    return Ok(());
                }
            };

            if self.dms_state.conversations.is_empty() {
                self.dms_state.error = Some("No conversations available.".to_string());
                return Ok(());
            }

            let conversation = &self.dms_state.conversations[selected_index];
            conversation.other_username.clone()
        };
        // Parse emoji shortcodes before sending
        let parsed_content = crate::emoji::parse_emoji_shortcodes(&content);

        match self
            .api_client
            .send_message(to_username.clone(), parsed_content)
            .await
        {
            Ok(_) => {
                self.clear_dm_message();
                self.input_mode = InputMode::Navigation;

                // If this was a pending conversation, clear it and reload conversations
                if self.dms_state.pending_conversation_username.is_some() {
                    self.dms_state.pending_conversation_username = None;
                    self.load_conversations().await?;
                    // Select the new conversation (will be first)
                    if !self.dms_state.conversations.is_empty() {
                        self.dms_state.selected_conversation_index = Some(0);
                        self.dms_state.needs_message_load = true;
                    }
                } else {
                    // Regular message in existing conversation
                    self.load_conversation_messages().await?;

                    // Keep unread count at 0 for current conversation
                    if let Some(index) = self.dms_state.selected_conversation_index {
                        if index < self.dms_state.conversations.len() {
                            let other_user_id = self.dms_state.conversations[index].other_user_id;
                            self.dms_state.unread_counts.insert(other_user_id, 0);
                        }
                    }
                }
            }
            Err(e) => {
                let error_str = e.to_string();
                // Check if this is a "user not found" error
                if error_str.contains("404")
                    || error_str.contains("not found")
                    || error_str.contains("User not found")
                {
                    // Show DM error modal with friend suggestion
                    self.dms_state.show_dm_error_modal = true;
                    self.dms_state.dm_error_message = format!(
                        "User '@{}' not found. Add them as a friend first?",
                        to_username
                    );
                    self.dms_state.failed_username = Some(to_username);
                    // Clear the pending conversation
                    self.dms_state.pending_conversation_username = None;
                } else {
                    let error_msg = categorize_error(&error_str);
                    self.dms_state.error = Some(error_msg);
                }
            }
        }

        Ok(())
    }

    /// Load mutual friends for DM availability
    pub async fn load_mutual_friends_for_dms(&mut self) -> Result<()> {
        match self.api_client.get_mutual_friends_list().await {
            Ok(friends) => {
                self.dms_state.available_mutual_friends =
                    friends.into_iter().map(|f| f.username).collect();
            }
            Err(e) => {
                // Don't block the modal from opening, just clear the list
                self.dms_state.available_mutual_friends.clear();
                eprintln!("Failed to load mutual friends: {}", e);
            }
        }
        Ok(())
    }

    /// Close new conversation modal
    pub fn close_new_conversation_modal(&mut self) {
        self.dms_state.show_new_conversation_modal = false;
        self.dms_state.new_conversation_username.clear();
        self.input_mode = InputMode::Navigation;
    }

    /// Add character to new conversation username
    pub fn add_char_to_new_conversation(&mut self, c: char) {
        if self.dms_state.new_conversation_username.len() < 50 {
            self.dms_state.new_conversation_username.push(c);
        }
    }

    /// Remove character from new conversation username
    pub fn remove_char_from_new_conversation(&mut self) {
        self.dms_state.new_conversation_username.pop();
    }

    /// Start new conversation (just prepare, don't send anything yet)
    pub async fn start_new_conversation(&mut self) -> Result<()> {
        if self.dms_state.new_conversation_username.trim().is_empty() {
            return Ok(());
        }

        self.dms_state.error = None;
        let to_username = self.dms_state.new_conversation_username.clone();

        // Validate that the user is a mutual friend
        if !self
            .dms_state
            .available_mutual_friends
            .contains(&to_username)
        {
            self.dms_state.error = Some(format!(
                "Cannot message '{}': You can only message mutual friends",
                to_username
            ));
            return Ok(());
        }

        // Store the username for the pending conversation
        self.dms_state.pending_conversation_username = Some(to_username.clone());

        // Close modal and clear messages (show empty conversation)
        self.close_new_conversation_modal();
        self.dms_state.messages.clear();

        // Set selection to None so the pending conversation is selected in the UI
        // (None means the pending conversation is active)
        self.dms_state.selected_conversation_index = None;

        // Switch to typing mode so user can immediately start composing
        self.input_mode = InputMode::Typing;

        Ok(())
    }

    /// Open existing conversation or create new one (from profile view)
    pub async fn open_or_create_dm_conversation(
        &mut self,
        username: String,
        user_id_str: String,
    ) -> Result<()> {
        // Parse user ID
        let user_id = match uuid::Uuid::parse_str(&user_id_str) {
            Ok(id) => id,
            Err(_) => {
                self.dms_state.error = Some("Invalid user ID".to_string());
                return Ok(());
            }
        };

        // Load conversations if not already loaded
        if self.dms_state.conversations.is_empty() {
            self.load_conversations().await?;
        }

        // Check if conversation already exists
        if let Some(index) = self
            .dms_state
            .conversations
            .iter()
            .position(|c| c.other_user_id == user_id)
        {
            // Conversation exists - select it
            self.dms_state.selected_conversation_index = Some(index);
            self.dms_state.pending_conversation_username = None;

            // Load messages for this conversation
            self.dms_state.needs_message_load = true;
            self.load_conversation_messages().await?;

            // Mark as read
            self.mark_conversation_as_read(user_id).await?;

            // Switch to typing mode
            self.input_mode = InputMode::Typing;
        } else {
            // Conversation doesn't exist - create new one
            self.dms_state.pending_conversation_username = Some(username);
            self.dms_state.messages.clear();
            self.dms_state.selected_conversation_index = None;

            // Switch to typing mode so user can start composing
            self.input_mode = InputMode::Typing;
        }

        Ok(())
    }

    /// Load settings
    pub async fn load_settings(&mut self) -> Result<()> {
        self.settings_state.loading = true;
        self.settings_state.error = None;

        match self.api_client.get_config().await {
            Ok(config) => {
                self.settings_state.max_posts_input = config.max_posts_display.to_string();
                self.settings_state.original_max_posts_input = config.max_posts_display.to_string();
                self.settings_state.config = Some(config.clone());
                self.settings_state.original_config = Some(config);
                self.settings_state.loading = false;
                self.settings_state.has_unsaved_changes = false;
            }
            Err(e) => {
                let error_msg = categorize_error(&e.to_string());
                self.settings_state.error = Some(error_msg);
                self.settings_state.loading = false;
            }
        }

        Ok(())
    }

    /// Cycle color scheme
    pub fn cycle_color_scheme(&mut self) {
        if let Some(config) = &mut self.settings_state.config {
            config.color_scheme = match config.color_scheme {
                fido_types::ColorScheme::Default => fido_types::ColorScheme::Dark,
                fido_types::ColorScheme::Dark => fido_types::ColorScheme::Light,
                fido_types::ColorScheme::Light => fido_types::ColorScheme::Solarized,
                fido_types::ColorScheme::Solarized => fido_types::ColorScheme::Default,
            };
            self.check_settings_changes();
        }
    }

    /// Cycle color scheme backward
    pub fn cycle_color_scheme_backward(&mut self) {
        if let Some(config) = &mut self.settings_state.config {
            config.color_scheme = match config.color_scheme {
                fido_types::ColorScheme::Default => fido_types::ColorScheme::Solarized,
                fido_types::ColorScheme::Dark => fido_types::ColorScheme::Default,
                fido_types::ColorScheme::Light => fido_types::ColorScheme::Dark,
                fido_types::ColorScheme::Solarized => fido_types::ColorScheme::Light,
            };
            self.check_settings_changes();
        }
    }

    /// Cycle sort order
    pub fn cycle_sort_order(&mut self) {
        if let Some(config) = &mut self.settings_state.config {
            config.sort_order = match config.sort_order {
                fido_types::SortOrder::Newest => fido_types::SortOrder::Popular,
                fido_types::SortOrder::Popular => fido_types::SortOrder::Controversial,
                fido_types::SortOrder::Controversial => fido_types::SortOrder::Newest,
            };
            self.check_settings_changes();
        }
    }

    /// Cycle sort order backward
    pub fn cycle_sort_order_backward(&mut self) {
        if let Some(config) = &mut self.settings_state.config {
            config.sort_order = match config.sort_order {
                fido_types::SortOrder::Newest => fido_types::SortOrder::Controversial,
                fido_types::SortOrder::Popular => fido_types::SortOrder::Newest,
                fido_types::SortOrder::Controversial => fido_types::SortOrder::Popular,
            };
            self.check_settings_changes();
        }
    }

    /// Add digit to max posts input
    pub fn add_digit_to_max_posts(&mut self, c: char) {
        if c.is_ascii_digit() && self.settings_state.max_posts_input.len() < 4 {
            self.settings_state.max_posts_input.push(c);
            self.check_settings_changes();
        }
    }

    /// Remove digit from max posts input
    pub fn remove_digit_from_max_posts(&mut self) {
        if !self.settings_state.max_posts_input.is_empty() {
            self.settings_state.max_posts_input.pop();
            self.check_settings_changes();
        }
    }

    /// Increment max posts display
    pub fn increment_max_posts(&mut self) {
        if let Ok(current) = self.settings_state.max_posts_input.parse::<i32>() {
            let new_value = (current + 1).min(9999); // Increment by 1, max 9999
            self.settings_state.max_posts_input = new_value.to_string();
            self.check_settings_changes();
        }
    }

    /// Decrement max posts display
    pub fn decrement_max_posts(&mut self) {
        if let Ok(current) = self.settings_state.max_posts_input.parse::<i32>() {
            let new_value = (current - 1).max(1); // Decrement by 1, min 1
            self.settings_state.max_posts_input = new_value.to_string();
            self.check_settings_changes();
        }
    }

    /// Save settings
    pub async fn save_settings(&mut self) -> Result<()> {
        if let Some(config) = &self.settings_state.config {
            self.settings_state.error = None;

            // Validate max posts
            let max_posts = match self.settings_state.max_posts_input.parse::<i32>() {
                Ok(n) if n > 0 => n,
                Ok(n) => {
                    self.settings_state.error = Some(format!(
                        "Validation Error: Max posts must be positive (got {})",
                        n
                    ));
                    return Ok(());
                }
                Err(_) if self.settings_state.max_posts_input.is_empty() => {
                    self.settings_state.error =
                        Some("Validation Error: Max posts cannot be empty".to_string());
                    return Ok(());
                }
                Err(_) => {
                    self.settings_state.error = Some(format!(
                        "Validation Error: '{}' is not a valid number",
                        self.settings_state.max_posts_input
                    ));
                    return Ok(());
                }
            };

            let request = fido_types::UpdateConfigRequest {
                color_scheme: Some(config.color_scheme.as_str().to_string()),
                sort_order: Some(config.sort_order.as_str().to_string()),
                max_posts_display: Some(max_posts),
                emoji_enabled: Some(config.emoji_enabled),
            };

            match self.api_client.update_config(request).await {
                Ok(updated_config) => {
                    self.settings_state.max_posts_input =
                        updated_config.max_posts_display.to_string();
                    self.settings_state.original_max_posts_input =
                        updated_config.max_posts_display.to_string();
                    self.settings_state.config = Some(updated_config.clone());
                    self.settings_state.original_config = Some(updated_config);
                    self.settings_state.has_unsaved_changes = false;
                    self.settings_state.error = Some("✓ Settings saved successfully!".to_string());

                    // Reload posts with new settings (max_posts_display and sort_order)
                    let _ = self.load_posts().await;
                }
                Err(e) => {
                    self.settings_state.error = Some(format!(
                        "Network Error: Failed to save settings: {} (Press 's' to retry)",
                        e
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if settings have changed from original
    fn check_settings_changes(&mut self) {
        if let (Some(current), Some(original)) = (
            &self.settings_state.config,
            &self.settings_state.original_config,
        ) {
            let config_changed = current.color_scheme != original.color_scheme
                || current.sort_order != original.sort_order;
            let max_posts_changed =
                self.settings_state.max_posts_input != self.settings_state.original_max_posts_input;

            self.settings_state.has_unsaved_changes = config_changed || max_posts_changed;
        }
    }

    /// Handle keyboard events with priority-based Escape handling
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        handlers::handle_key_event(self, key)
    }

    /// Handle keys for main screen
    pub fn handle_main_keys(&mut self, key: KeyEvent) -> Result<()> {
        handlers::handle_main_keys(self, key)
    }

    /// Handle keys for Profile tab
    pub fn handle_profile_keys(&mut self, key: KeyEvent) -> Result<()> {
        // If bio edit modal is open, handle modal keys
        if self.profile_state.show_edit_bio_modal {
            return self.handle_edit_bio_modal_keys(key);
        }

        // Normal profile tab keys
        match key.code {
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => self.next_user_post(),
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => self.previous_user_post(),
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if let Some(profile) = &self.profile_state.profile {
                    let current_bio = profile.bio.clone().unwrap_or_default();
                    self.open_composer_edit_bio(current_bio);
                }
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Open social connections modal
                self.friends_state.show_friends_modal = true;
                self.friends_state.selected_index = 0;
                self.friends_state.error = None;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys for edit bio modal
    pub fn handle_edit_bio_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.add_char_to_bio(c);
            }
            KeyCode::Backspace => {
                self.remove_char_from_bio();
            }
            KeyCode::Enter => {
                // Enter saves bio (will be handled async in main loop)
            }
            KeyCode::Left => {
                if self.profile_state.edit_bio_cursor_position > 0 {
                    self.profile_state.edit_bio_cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                let char_count = self.profile_state.edit_bio_content.chars().count();
                if self.profile_state.edit_bio_cursor_position < char_count {
                    self.profile_state.edit_bio_cursor_position += 1;
                }
            }
            KeyCode::Home => {
                // Move to start of text
                self.profile_state.edit_bio_cursor_position = 0;
            }
            KeyCode::End => {
                // Move to end of text (character count, not byte length)
                self.profile_state.edit_bio_cursor_position =
                    self.profile_state.edit_bio_content.chars().count();
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys for DMs tab
    pub fn handle_dms_keys(&mut self, key: KeyEvent) -> Result<()> {
        // If new conversation modal is open, handle modal keys
        if self.dms_state.show_new_conversation_modal {
            return self.handle_new_conversation_modal_keys(key);
        }

        // Check input mode
        match self.input_mode {
            InputMode::Navigation => {
                match key.code {
                    // Navigation
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                        match self.dms_state.selected_conversation_index {
                            None => {
                                // No selection, select "New Conversation" button (index usize::MAX)
                                self.dms_state.selected_conversation_index = Some(usize::MAX);
                            }
                            Some(usize::MAX) => {
                                // On "New Conversation" button, move to first conversation
                                if !self.dms_state.conversations.is_empty() {
                                    self.dms_state.selected_conversation_index = Some(0);
                                }
                            }
                            Some(index) => {
                                // Move down if not at bottom
                                if index < self.dms_state.conversations.len().saturating_sub(1) {
                                    self.dms_state.selected_conversation_index = Some(index + 1);
                                }
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                        match self.dms_state.selected_conversation_index {
                            None => {
                                // No selection, select last conversation or "New Conversation" button
                                if !self.dms_state.conversations.is_empty() {
                                    self.dms_state.selected_conversation_index =
                                        Some(self.dms_state.conversations.len() - 1);
                                } else {
                                    self.dms_state.selected_conversation_index = Some(usize::MAX);
                                }
                            }
                            Some(0) => {
                                // At first conversation, move to "New Conversation" button
                                self.dms_state.selected_conversation_index = Some(usize::MAX);
                            }
                            Some(usize::MAX) => {
                                // At "New Conversation" button, can't go higher
                            }
                            Some(index) => {
                                // Move up
                                self.dms_state.selected_conversation_index = Some(index - 1);
                            }
                        }
                    }
                    KeyCode::Enter => {
                        // If "New Conversation" button is selected, open modal
                        // Note: Actual modal opening with data fetch happens in main.rs event loop
                        if self.dms_state.selected_conversation_index == Some(usize::MAX) {
                            // Set a flag to trigger async modal opening in main loop
                            self.dms_state.show_new_conversation_modal = true;
                            self.dms_state.new_conversation_username.clear();
                            self.input_mode = InputMode::Typing;
                        }
                    }

                    _ => {
                        // Any other key starts typing mode
                        self.input_mode = InputMode::Typing;
                        self.handle_dm_input(key);
                        // Trigger message load if not already loaded for this conversation
                        if self.dms_state.messages.is_empty() {
                            self.dms_state.needs_message_load = true;
                        }
                    }
                }
            }
            InputMode::Typing => {
                match key.code {
                    KeyCode::Esc => {
                        // Clear input and return to navigation mode
                        self.clear_dm_message();
                        self.input_mode = InputMode::Navigation;
                    }
                    KeyCode::Enter => {
                        // Send message (will be handled async in main loop)
                        // Mode will switch back to navigation after successful send
                    }
                    _ => {
                        // Let TextArea handle all other keys
                        self.handle_dm_input(key);
                        // Switch back to navigation if input becomes empty after backspace
                        if self.is_dm_message_empty() && key.code == KeyCode::Backspace {
                            self.input_mode = InputMode::Navigation;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle keys for new conversation modal
    pub fn handle_new_conversation_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.add_char_to_new_conversation(c);
            }
            KeyCode::Backspace => {
                self.remove_char_from_new_conversation();
            }
            KeyCode::Enter => {
                // Start conversation (will be handled async in main loop)
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys for Settings tab
    pub fn handle_settings_keys(&mut self, key: KeyEvent) -> Result<()> {
        // If save confirmation is showing, handle that
        if self.settings_state.show_save_confirmation {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    // Save and switch tabs (will be handled async in main loop)
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.confirm_discard_changes();
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.settings_state.selected_field = match self.settings_state.selected_field {
                    SettingsField::ColorScheme => SettingsField::SortOrder,
                    SettingsField::SortOrder => SettingsField::MaxPosts,
                    SettingsField::MaxPosts => SettingsField::MaxPosts, // Stop at last field
                };
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.settings_state.selected_field = match self.settings_state.selected_field {
                    SettingsField::ColorScheme => SettingsField::ColorScheme, // Stop at first field
                    SettingsField::SortOrder => SettingsField::ColorScheme,
                    SettingsField::MaxPosts => SettingsField::SortOrder,
                };
            }
            KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Left => match self.settings_state.selected_field {
                SettingsField::ColorScheme => self.cycle_color_scheme_backward(),
                SettingsField::SortOrder => self.cycle_sort_order_backward(),
                SettingsField::MaxPosts => self.decrement_max_posts(),
            },
            KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Right | KeyCode::Enter => match self.settings_state.selected_field {
                SettingsField::ColorScheme => self.cycle_color_scheme(),
                SettingsField::SortOrder => self.cycle_sort_order(),
                SettingsField::MaxPosts => self.increment_max_posts(),
            },
            KeyCode::Backspace if self.settings_state.selected_field == SettingsField::MaxPosts => {
                self.remove_digit_from_max_posts();
            }
            KeyCode::Char(c) if self.settings_state.selected_field == SettingsField::MaxPosts => {
                self.add_digit_to_max_posts(c);
            }
            KeyCode::Char('s') => {
                // Save settings (will be handled async in main loop)
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys for Posts tab
    pub fn handle_posts_keys(&mut self, key: KeyEvent) -> Result<()> {
        handlers::handle_posts_keys(self, key)
    }

    /// Handle keys for filter modal
    pub fn handle_filter_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        handlers::handle_filter_modal_keys(self, key)
    }

    /// Handle keys for post detail view (now always modal-based)
    pub fn handle_post_detail_keys(&mut self, key: KeyEvent) -> Result<()> {
        // Since we always open modal directly now, just route to modal handler
        // The modal is always open when viewing_post_detail is true
        self.handle_full_post_modal_keys(key)
    }

    /// Handle keys for full post modal
    pub fn handle_full_post_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.close_full_post_modal();
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.modal_next_reply();
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.modal_previous_reply();
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                // Toggle expansion in modal
                self.modal_toggle_expansion();
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Reply to the currently selected post/comment in modal
                if let Some(detail_state) = &self.post_detail_state {
                    let selected_idx = detail_state.modal_list_state.selected().unwrap_or(0);
                    let modal_root_id = detail_state.full_post_modal_id;
                    
                    // Find the selected post (index 0 = root, 1+ = replies)
                    let post_to_reply = if selected_idx == 0 {
                        // Replying to root post
                        if let Some(root_id) = modal_root_id {
                            if let Some(post) = &detail_state.post {
                                if post.id == root_id {
                                    Some(post.clone())
                                } else {
                                    detail_state.replies.iter().find(|r| r.id == root_id).cloned()
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        // Replying to a reply - need to find which one based on flattened tree
                        self.get_selected_post_in_modal()
                    };

                    if let Some(post) = post_to_reply {
                        // DON'T close the post modal - keep it visible in the background
                        // The composer will render on top of it (like profile modal does)
                        // User can press Esc to close composer and return to modal
                        
                        self.open_composer_reply(
                            post.id,
                            post.author_username.clone(),
                            post.content.clone(),
                        );
                    }
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // Comment on selected reply in modal (nested reply)
                // This will be handled by finding the selected item in the modal tree
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                // Upvote from modal (will be handled async in main loop)
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // Downvote from modal (will be handled async in main loop)
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                // Delete selected post (only if user owns it)
                self.show_delete_confirmation();
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // View profile of selected post author (will be handled async in main loop)
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys for reply composer
    pub fn handle_reply_composer_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.add_char_to_reply(c);
            }
            KeyCode::Backspace => {
                self.remove_char_from_reply();
            }
            KeyCode::Enter
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                // Submit reply (will be handled async in main loop)
            }
            KeyCode::Esc => {
                self.close_reply_composer();
            }
            _ => {}
        }
        Ok(())
    }

    pub fn next_post(&mut self) {
        if self.posts_state.posts.is_empty() {
            return;
        }

        // Get current post index (not list index)
        let current_post_index = self
            .posts_state
            .list_state
            .selected()
            .and_then(|list_idx| self.posts_state.list_index_to_post_index(list_idx));

        let next_post_index = match current_post_index {
            Some(i) => {
                // Stop at bottom, don't wrap around
                if i >= self.posts_state.posts.len() - 1 {
                    // At last post - show "End of Feed" indicator
                    self.posts_state.at_end_of_feed = true;
                    i
                } else {
                    self.posts_state.at_end_of_feed = false;
                    i + 1
                }
            }
            None => {
                self.posts_state.at_end_of_feed = false;
                0
            }
        };

        // Convert post index to list index and update selection
        let list_index = self.posts_state.post_index_to_list_index(next_post_index);
        self.posts_state.list_state.select(Some(list_index));
    }

    pub fn previous_post(&mut self) {
        if self.posts_state.posts.is_empty() {
            return;
        }

        // Clear end-of-feed indicator when scrolling up
        self.posts_state.at_end_of_feed = false;

        let current = self.posts_state.list_state.selected();

        match current {
            Some(i) if i > 0 => {
                self.posts_state.list_state.select(Some(i - 1));
            }
            _ => {
                // Already at top or no selection
                self.posts_state.list_state.select(Some(0));
            }
        }
    }

    pub fn next_user_post(&mut self) {
        if self.profile_state.user_posts.is_empty() {
            return;
        }
        let i = match self.profile_state.list_state.selected() {
            Some(i) => {
                if i >= self.profile_state.user_posts.len() - 1 {
                    i // Stop at last post, don't wrap
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.profile_state.list_state.select(Some(i));
    }

    pub fn previous_user_post(&mut self) {
        if self.profile_state.user_posts.is_empty() {
            return;
        }
        let i = match self.profile_state.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0 // Stop at first post, don't wrap
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.profile_state.list_state.select(Some(i));
    }

    /// Handle keys for new post modal
    pub fn handle_new_post_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.add_char_to_post(c);
            }
            KeyCode::Backspace => {
                self.remove_char_from_post();
            }
            KeyCode::Enter => {
                // Don't add newline - Ctrl+Enter is handled in main loop for submission
                // Regular Enter does nothing in single-line post input
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys for authentication screen
    pub fn handle_auth_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.auth_state.selected_index > 0 {
                    self.auth_state.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                // Limit to 3 test users (alice, bob, charlie)
                let max_users = self.auth_state.test_users.len().min(3);
                if self.auth_state.selected_index < max_users.saturating_sub(1) {
                    self.auth_state.selected_index += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Poll for events with timeout
    pub fn poll_event(&mut self, timeout: Duration) -> Result<bool> {
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Open post detail view
    pub async fn open_post_detail(&mut self, post_id: Uuid) -> Result<()> {
        let previous_position = self.posts_state.list_state.selected();
        
        // Initialize modal list state with root post selected (index 0)
        let mut modal_list_state = ListState::default();
        modal_list_state.select(Some(0));
        
        // Pre-expand the root post to show first layer of comments
        let mut modal_expanded_posts = std::collections::HashMap::new();
        modal_expanded_posts.insert(post_id, true);
        
        self.post_detail_state = Some(PostDetailState {
            post: None,
            replies: Vec::new(),
            reply_list_state: ListState::default(),
            loading: true,
            error: None,
            message: None,
            show_reply_composer: false,
            reply_content: String::new(),
            show_delete_confirmation: false,
            previous_feed_position: previous_position,
            expanded_posts: std::collections::HashMap::new(),
            show_full_post_modal: true, // Open modal directly
            full_post_modal_id: Some(post_id), // Set the post ID for modal
            modal_list_state,
            modal_expanded_posts, // Root post pre-expanded
        });
        self.viewing_post_detail = true;
        self.load_post_detail(post_id).await?;
        Ok(())
    }

    async fn load_post_detail(&mut self, post_id: Uuid) -> Result<()> {
        // If post_detail_state doesn't exist, create it first
        if self.post_detail_state.is_none() {
            let mut modal_list_state = ListState::default();
            modal_list_state.select(Some(0));
            
            let mut modal_expanded_posts = std::collections::HashMap::new();
            modal_expanded_posts.insert(post_id, true);
            
            self.post_detail_state = Some(PostDetailState {
                post: None,
                replies: Vec::new(),
                reply_list_state: ListState::default(),
                loading: true,
                error: None,
                message: None,
                show_reply_composer: false,
                reply_content: String::new(),
                show_delete_confirmation: false,
                previous_feed_position: self.posts_state.list_state.selected(),
                expanded_posts: std::collections::HashMap::new(),
                show_full_post_modal: true,
                full_post_modal_id: Some(post_id),
                modal_list_state,
                modal_expanded_posts,
            });
        }
        
        if let Some(detail_state) = &mut self.post_detail_state {
            // Preserve ALL state before reloading (including the post itself)
            let was_modal_open = detail_state.show_full_post_modal;
            let modal_post_id = detail_state.full_post_modal_id;
            let modal_selection = detail_state.modal_list_state.selected();
            let modal_expanded = detail_state.modal_expanded_posts.clone();
            let old_post = detail_state.post.clone(); // Preserve old post data
            
            detail_state.loading = true;
            detail_state.error = None;
            
            // Fetch new post data
            match self.api_client.get_post_by_id(post_id).await {
                Ok(post) => detail_state.post = Some(post),
                Err(e) => {
                    // On error, restore old post so modal can still render
                    detail_state.post = old_post;
                    detail_state.error = Some(categorize_error(&e.to_string()));
                    detail_state.loading = false;
                    return Ok(());
                }
            }
            
            // Fetch replies
            match self.api_client.get_replies(post_id).await {
                Ok(replies) => {
                    detail_state.replies = replies;
                    // Initialize reply list state - select first reply if any exist
                    if !detail_state.replies.is_empty() {
                        detail_state.reply_list_state.select(Some(0));
                    } else {
                        detail_state.reply_list_state.select(None);
                    }
                    detail_state.loading = false;
                    
                    // Restore modal state after reload
                    if was_modal_open {
                        detail_state.show_full_post_modal = true;
                        detail_state.full_post_modal_id = modal_post_id;
                        detail_state.modal_list_state.select(modal_selection);
                        detail_state.modal_expanded_posts = modal_expanded;
                    }
                }
                Err(e) => {
                    detail_state.error = Some(categorize_error(&e.to_string()));
                    detail_state.loading = false;
                }
            }
        }
        Ok(())
    }

    pub fn close_post_detail(&mut self) {
        if let Some(detail_state) = &self.post_detail_state {
            if let Some(position) = detail_state.previous_feed_position {
                self.posts_state.list_state.select(Some(position));
            }
        }
        // Clean up all modal state
        self.post_detail_state = None;
        self.viewing_post_detail = false;
        self.input_mode = InputMode::Navigation;
    }

    pub async fn vote_in_detail_view(&mut self, direction: &str) -> Result<()> {
        let detail_state = match &mut self.post_detail_state {
            Some(state) => state,
            None => return Ok(()),
        };
        detail_state.error = None;

        // Check if modal is open - if so, handle modal voting
        if detail_state.show_full_post_modal {
            return self.vote_in_modal(direction).await;
        }

        let (post_id, is_reply, reply_index) = if let Some(selected_idx) =
            detail_state.reply_list_state.selected()
        {
            // Get direct replies to find the actual reply
            let direct_replies: Vec<&Post> = detail_state
                .replies
                .iter()
                .filter(|reply| {
                    if let Some(parent_id) = reply.parent_post_id {
                        !detail_state.replies.iter().any(|r| r.id == parent_id)
                    } else {
                        false
                    }
                })
                .collect();

            if let Some(reply) = direct_replies.get(selected_idx) {
                // Find the actual index in the full replies vec
                if let Some(actual_idx) = detail_state.replies.iter().position(|r| r.id == reply.id)
                {
                    (reply.id, true, Some(actual_idx))
                } else {
                    return Ok(());
                }
            } else {
                return Ok(());
            }
        } else {
            match &detail_state.post {
                Some(post) => (post.id, false, None),
                None => return Ok(()),
            }
        };
        let (previous_vote, original_upvotes, original_downvotes) = if is_reply {
            let reply = &detail_state.replies[reply_index.unwrap()];
            (reply.user_vote.clone(), reply.upvotes, reply.downvotes)
        } else {
            let post = detail_state.post.as_ref().unwrap();
            (post.user_vote.clone(), post.upvotes, post.downvotes)
        };
        if let Some(ref prev_direction) = previous_vote {
            if prev_direction == direction {
                return Ok(());
            }
        }
        if is_reply {
            let reply = &mut detail_state.replies[reply_index.unwrap()];
            match (&previous_vote, direction) {
                (None, "up") => {
                    reply.upvotes += 1;
                    reply.user_vote = Some("up".to_string());
                }
                (None, "down") => {
                    reply.downvotes += 1;
                    reply.user_vote = Some("down".to_string());
                }
                (Some(prev), "up") if prev == "down" => {
                    reply.downvotes -= 1;
                    reply.upvotes += 1;
                    reply.user_vote = Some("up".to_string());
                }
                (Some(prev), "down") if prev == "up" => {
                    reply.upvotes -= 1;
                    reply.downvotes += 1;
                    reply.user_vote = Some("down".to_string());
                }
                _ => {}
            }
        } else {
            let post = detail_state.post.as_mut().unwrap();
            match (&previous_vote, direction) {
                (None, "up") => {
                    post.upvotes += 1;
                    post.user_vote = Some("up".to_string());
                }
                (None, "down") => {
                    post.downvotes += 1;
                    post.user_vote = Some("down".to_string());
                }
                (Some(prev), "up") if prev == "down" => {
                    post.downvotes -= 1;
                    post.upvotes += 1;
                    post.user_vote = Some("up".to_string());
                }
                (Some(prev), "down") if prev == "up" => {
                    post.upvotes -= 1;
                    post.downvotes += 1;
                    post.user_vote = Some("down".to_string());
                }
                _ => {}
            }
        }
        match self
            .api_client
            .vote_on_post(post_id, direction.to_string())
            .await
        {
            Ok(_) => {}
            Err(e) => {
                let detail_state = self.post_detail_state.as_mut().unwrap();
                if is_reply {
                    let reply = &mut detail_state.replies[reply_index.unwrap()];
                    reply.upvotes = original_upvotes;
                    reply.downvotes = original_downvotes;
                    reply.user_vote = previous_vote;
                } else {
                    let post = detail_state.post.as_mut().unwrap();
                    post.upvotes = original_upvotes;
                    post.downvotes = original_downvotes;
                    post.user_vote = previous_vote;
                }
                detail_state.error = Some(categorize_error(&e.to_string()));
            }
        }
        Ok(())
    }

    /// Vote on a post in the full post modal
    /// The modal tracks which post is selected via modal_list_state
    /// Index 0 = root post, Index 1+ = flattened visible replies
    pub async fn vote_in_modal(&mut self, direction: &str) -> Result<()> {
        let detail_state = match &mut self.post_detail_state {
            Some(state) => state,
            None => return Ok(()),
        };

        // Get the modal root post ID
        let modal_root_id = match detail_state.full_post_modal_id {
            Some(id) => id,
            None => return Ok(()),
        };

        // Get selected index (0 = root, 1+ = replies)
        let selected_idx = detail_state.modal_list_state.selected().unwrap_or(0);

        // Find the post ID to vote on
        let post_id = if selected_idx == 0 {
            // Voting on root post
            modal_root_id
        } else {
            // Voting on a reply - need to map index to post ID
            // Find the modal root post
            let modal_root = if let Some(post) = &detail_state.post {
                if post.id == modal_root_id {
                    Some(post.clone())
                } else {
                    detail_state
                        .replies
                        .iter()
                        .find(|r| r.id == modal_root_id)
                        .cloned()
                }
            } else {
                None
            };

            if let Some(root) = modal_root {
                // Filter replies to descendants (excluding the root itself)
                let modal_replies: Vec<Post> = detail_state
                    .replies
                    .iter()
                    .filter(|reply| {
                        if reply.id == root.id {
                            return false;
                        }
                        let mut current_parent = reply.parent_post_id;
                        while let Some(parent_id) = current_parent {
                            if parent_id == root.id {
                                return true;
                            }
                            current_parent = detail_state
                                .replies
                                .iter()
                                .find(|r| r.id == parent_id)
                                .and_then(|r| r.parent_post_id);
                        }
                        false
                    })
                    .cloned()
                    .collect();

                // Build tree and flatten to map index to post
                use std::collections::HashMap;
                let mut children_map: HashMap<Uuid, Vec<&Post>> = HashMap::new();
                for reply in &modal_replies {
                    if let Some(parent_id) = reply.parent_post_id {
                        children_map
                            .entry(parent_id)
                            .or_default()
                            .push(reply);
                    }
                }

                // Flatten the tree respecting current expansion state
                let mut flattened_posts = vec![];
                fn collect_visible(
                    post_id: &Uuid,
                    children_map: &HashMap<Uuid, Vec<&Post>>,
                    expanded_posts: &HashMap<Uuid, bool>,
                    result: &mut Vec<Uuid>,
                ) {
                    if let Some(children) = children_map.get(post_id) {
                        for child in children {
                            result.push(child.id);
                            if expanded_posts.get(&child.id).copied().unwrap_or(false) {
                                collect_visible(&child.id, children_map, expanded_posts, result);
                            }
                        }
                    }
                }

                collect_visible(
                    &root.id,
                    &children_map,
                    &detail_state.modal_expanded_posts,
                    &mut flattened_posts,
                );

                // Map selected_idx to post_id (accounting for root at index 0)
                if selected_idx > 0 && selected_idx <= flattened_posts.len() {
                    flattened_posts[selected_idx - 1]
                } else {
                    return Ok(()); // Invalid index
                }
            } else {
                return Ok(()); // Root not found
            }
        };

        // Find the post to get its current vote state
        let (previous_vote, original_upvotes, original_downvotes) =
            if let Some(post) = &detail_state.post {
                if post.id == post_id {
                    (post.user_vote.clone(), post.upvotes, post.downvotes)
                } else if let Some(reply) = detail_state.replies.iter().find(|r| r.id == post_id) {
                    (reply.user_vote.clone(), reply.upvotes, reply.downvotes)
                } else {
                    return Ok(());
                }
            } else if let Some(reply) = detail_state.replies.iter().find(|r| r.id == post_id) {
                (reply.user_vote.clone(), reply.upvotes, reply.downvotes)
            } else {
                return Ok(());
            };

        // Check if user is trying to vote the same direction again
        if let Some(ref prev_direction) = previous_vote {
            if prev_direction == direction {
                return Ok(());
            }
        }

        // Optimistic update - find and update the post
        let detail_state = self.post_detail_state.as_mut().unwrap();

        // Check if it's the main post
        if let Some(post) = &mut detail_state.post {
            if post.id == post_id {
                match (&previous_vote, direction) {
                    (None, "up") => {
                        post.upvotes += 1;
                        post.user_vote = Some("up".to_string());
                    }
                    (None, "down") => {
                        post.downvotes += 1;
                        post.user_vote = Some("down".to_string());
                    }
                    (Some(prev), "up") if prev == "down" => {
                        post.downvotes -= 1;
                        post.upvotes += 1;
                        post.user_vote = Some("up".to_string());
                    }
                    (Some(prev), "down") if prev == "up" => {
                        post.upvotes -= 1;
                        post.downvotes += 1;
                        post.user_vote = Some("down".to_string());
                    }
                    _ => {}
                }
            }
        }

        // Check if it's in replies
        if let Some(reply) = detail_state.replies.iter_mut().find(|r| r.id == post_id) {
            match (&previous_vote, direction) {
                (None, "up") => {
                    reply.upvotes += 1;
                    reply.user_vote = Some("up".to_string());
                }
                (None, "down") => {
                    reply.downvotes += 1;
                    reply.user_vote = Some("down".to_string());
                }
                (Some(prev), "up") if prev == "down" => {
                    reply.downvotes -= 1;
                    reply.upvotes += 1;
                    reply.user_vote = Some("up".to_string());
                }
                (Some(prev), "down") if prev == "up" => {
                    reply.upvotes -= 1;
                    reply.downvotes += 1;
                    reply.user_vote = Some("down".to_string());
                }
                _ => {}
            }
        }

        // Send vote to server
        match self
            .api_client
            .vote_on_post(post_id, direction.to_string())
            .await
        {
            Ok(_) => {
                // Success - optimistic update is already applied
            }
            Err(e) => {
                // Revert optimistic update on error
                let detail_state = self.post_detail_state.as_mut().unwrap();

                // Revert main post if needed
                if let Some(post) = &mut detail_state.post {
                    if post.id == post_id {
                        post.upvotes = original_upvotes;
                        post.downvotes = original_downvotes;
                        post.user_vote = previous_vote.clone();
                    }
                }

                // Revert reply if needed
                if let Some(reply) = detail_state.replies.iter_mut().find(|r| r.id == post_id) {
                    reply.upvotes = original_upvotes;
                    reply.downvotes = original_downvotes;
                    reply.user_vote = previous_vote;
                }

                detail_state.error = Some(categorize_error(&e.to_string()));
            }
        }

        Ok(())
    }

    pub fn open_reply_composer(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            detail_state.show_reply_composer = true;
            detail_state.reply_content.clear();
            self.input_mode = InputMode::Typing;
        }
    }

    pub fn close_reply_composer(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            detail_state.show_reply_composer = false;
            detail_state.reply_content.clear();
            self.input_mode = InputMode::Navigation;
        }
    }

    pub fn add_char_to_reply(&mut self, c: char) {
        if let Some(detail_state) = &mut self.post_detail_state {
            if detail_state.reply_content.len() < 280 {
                detail_state.reply_content.push(c);
            }
        }
    }

    pub fn remove_char_from_reply(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            detail_state.reply_content.pop();
        }
    }

    pub async fn submit_reply(&mut self) -> Result<()> {
        let detail_state = match &mut self.post_detail_state {
            Some(state) => state,
            None => return Ok(()),
        };
        let trimmed = detail_state.reply_content.trim();
        if trimmed.is_empty() {
            detail_state.error = Some(
                "Validation Error: Cannot post empty reply. Type something first!".to_string(),
            );
            return Ok(());
        }
        let char_count = crate::emoji::count_characters(&detail_state.reply_content);
        if char_count > 280 {
            detail_state.error = Some(format!(
                "Validation Error: Reply exceeds 280 characters (current: {})",
                char_count
            ));
            return Ok(());
        }
        let parent_post_id = match &detail_state.post {
            Some(post) => post.id,
            None => return Ok(()),
        };
        detail_state.error = None;
        let content = crate::emoji::parse_emoji_shortcodes(&detail_state.reply_content);
        match self.api_client.create_reply(parent_post_id, content).await {
            Ok(new_reply) => {
                if let Some(detail_state) = &mut self.post_detail_state {
                    detail_state.replies.push(new_reply);
                    if let Some(post) = &mut detail_state.post {
                        post.reply_count += 1;
                    }
                }
                self.close_reply_composer();
            }
            Err(e) => {
                if let Some(detail_state) = &mut self.post_detail_state {
                    detail_state.error = Some(categorize_error(&e.to_string()));
                }
            }
        }
        Ok(())
    }



    /// Get author ID from selected post in feed
    pub fn get_selected_post_author_id(&self) -> Option<String> {
        let selected = self.posts_state.list_state.selected()?;
        self.posts_state
            .posts
            .get(selected)
            .map(|p| p.author_id.to_string())
    }

    /// Get author ID from post detail view (main post or selected reply)
    pub fn get_post_detail_author_id(&self) -> Option<String> {
        let detail_state = self.post_detail_state.as_ref()?;

        if let Some(selected_idx) = detail_state.reply_list_state.selected() {
            // Get direct replies to find the selected one
            let direct_replies: Vec<&Post> = detail_state
                .replies
                .iter()
                .filter(|reply| {
                    if let Some(parent_id) = reply.parent_post_id {
                        !detail_state.replies.iter().any(|r| r.id == parent_id)
                    } else {
                        false
                    }
                })
                .collect();

            direct_replies
                .get(selected_idx)
                .map(|r| r.author_id.to_string())
        } else {
            // Get author from main post
            detail_state.post.as_ref().map(|p| p.author_id.to_string())
        }
    }

    /// Close user profile view
    pub fn close_user_profile_view(&mut self) {
        self.user_profile_view = None;

        // Reopen social modal if flag is set
        if self.friends_state.return_to_modal_after_profile {
            self.friends_state.show_friends_modal = true;
            self.friends_state.return_to_modal_after_profile = false;
        }
    }

    /// Toggle expansion of selected reply (for nested replies)
    pub fn toggle_reply_expansion(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            if let Some(selected_idx) = detail_state.reply_list_state.selected() {
                // Get direct replies to find the selected one
                let direct_replies: Vec<&Post> = detail_state
                    .replies
                    .iter()
                    .filter(|reply| {
                        if let Some(parent_id) = reply.parent_post_id {
                            !detail_state.replies.iter().any(|r| r.id == parent_id)
                        } else {
                            false
                        }
                    })
                    .collect();

                if let Some(reply) = direct_replies.get(selected_idx) {
                    let post_id = reply.id;
                    let is_expanded = detail_state
                        .expanded_posts
                        .get(&post_id)
                        .copied()
                        .unwrap_or(false);
                    detail_state.expanded_posts.insert(post_id, !is_expanded);
                }
            }
        }
    }

    /// Open full post modal for selected reply
    pub fn open_full_post_modal(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            if let Some(selected_idx) = detail_state.reply_list_state.selected() {
                // Find the direct reply at this index
                let direct_replies: Vec<&Post> = detail_state
                    .replies
                    .iter()
                    .filter(|reply| {
                        if let Some(parent_id) = reply.parent_post_id {
                            !detail_state.replies.iter().any(|r| r.id == parent_id)
                        } else {
                            false
                        }
                    })
                    .collect();

                if let Some(reply) = direct_replies.get(selected_idx) {
                    detail_state.full_post_modal_id = Some(reply.id);
                    detail_state.show_full_post_modal = true;
                    // Initialize modal state - select first item
                    detail_state.modal_list_state.select(Some(0));
                    // Start with everything collapsed (empty expansion state)
                    detail_state.modal_expanded_posts.clear();
                }
            } else if let Some(post) = &detail_state.post {
                // Open modal for main post if no reply selected
                detail_state.full_post_modal_id = Some(post.id);
                detail_state.show_full_post_modal = true;
                detail_state.modal_list_state.select(Some(0));
                // Start with everything collapsed
                detail_state.modal_expanded_posts.clear();
            }
        }
    }

    /// Close full post modal
    pub fn close_full_post_modal(&mut self) {
        // Since we're using modal-first approach, closing the modal means
        // closing the entire post detail view and returning to feed
        self.close_post_detail();
    }
    
    /// Get the currently selected post in the modal based on flattened tree
    fn get_selected_post_in_modal(&self) -> Option<Post> {
        let detail_state = self.post_detail_state.as_ref()?;
        let selected_idx = detail_state.modal_list_state.selected()?;
        
        if selected_idx == 0 {
            // Root post
            let root_id = detail_state.full_post_modal_id?;
            if let Some(post) = &detail_state.post {
                if post.id == root_id {
                    return Some(post.clone());
                }
            }
            return detail_state.replies.iter().find(|r| r.id == root_id).cloned();
        }
        
        // Find in flattened tree (selected_idx - 1 because root is index 0)
        let root_id = detail_state.full_post_modal_id?;
        let root_post = if let Some(post) = &detail_state.post {
            if post.id == root_id {
                post.clone()
            } else {
                detail_state.replies.iter().find(|r| r.id == root_id)?.clone()
            }
        } else {
            return None;
        };
        
        // Filter replies to descendants
        let modal_replies: Vec<Post> = detail_state.replies.iter()
            .filter(|reply| {
                reply.id != root_post.id && is_descendant_of_post(reply, &root_post.id, &detail_state.replies)
            })
            .cloned()
            .collect();
        
        // Build and flatten tree
        let flattened = self.flatten_modal_tree(&root_post, &modal_replies, &detail_state.modal_expanded_posts);
        
        // Get the post at selected_idx - 1
        flattened.get(selected_idx - 1).cloned()
    }
    
    /// Flatten modal tree for selection
    fn flatten_modal_tree(&self, root: &Post, replies: &[Post], expanded: &std::collections::HashMap<Uuid, bool>) -> Vec<Post> {
        use std::collections::HashMap;
        
        let mut children_map: HashMap<Uuid, Vec<&Post>> = HashMap::new();
        for reply in replies {
            if let Some(parent_id) = reply.parent_post_id {
                children_map.entry(parent_id).or_default().push(reply);
            }
        }
        
        let mut result = Vec::new();
        
        fn collect_visible(
            post_id: &Uuid,
            children_map: &HashMap<Uuid, Vec<&Post>>,
            expanded: &HashMap<Uuid, bool>,
            result: &mut Vec<Post>,
        ) {
            if let Some(children) = children_map.get(post_id) {
                for child in children {
                    result.push((*child).clone());
                    if expanded.get(&child.id).copied().unwrap_or(false) {
                        collect_visible(&child.id, children_map, expanded, result);
                    }
                }
            }
        }
        
        if expanded.get(&root.id).copied().unwrap_or(false) {
            collect_visible(&root.id, &children_map, expanded, &mut result);
        }
        
        result
    }

    /// Navigate down in modal
    pub fn modal_next_reply(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            if detail_state.show_full_post_modal {
                let current = detail_state.modal_list_state.selected().unwrap_or(0);

                // Calculate the actual max based on the flattened tree
                let max_index = Self::calculate_modal_max_index_for_state(detail_state);

                // Only increment if we're not at the last item
                if current < max_index {
                    let next_index = current + 1;
                    detail_state.modal_list_state.select(Some(next_index));
                }
            }
        }
    }

    /// Navigate up in modal
    pub fn modal_previous_reply(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            if detail_state.show_full_post_modal {
                let current = detail_state.modal_list_state.selected().unwrap_or(0);
                if current > 0 {
                    detail_state.modal_list_state.select(Some(current - 1));
                }
            }
        }
    }

    /// Calculate the maximum index for modal navigation
    fn calculate_modal_max_index_for_state(detail_state: &PostDetailState) -> usize {
        if let Some(modal_post_id) = detail_state.full_post_modal_id {
            // Find the modal root post
            let modal_root = if let Some(post) = &detail_state.post {
                if post.id == modal_post_id {
                    Some(post.clone())
                } else {
                    detail_state
                        .replies
                        .iter()
                        .find(|r| r.id == modal_post_id)
                        .cloned()
                }
            } else {
                None
            };

            if let Some(root) = modal_root {
                // Filter replies to descendants
                let modal_replies: Vec<Post> = detail_state
                    .replies
                    .iter()
                    .filter(|reply| {
                        if reply.id == root.id {
                            return false;
                        }
                        let mut current_parent = reply.parent_post_id;
                        while let Some(parent_id) = current_parent {
                            if parent_id == root.id {
                                return true;
                            }
                            current_parent = detail_state
                                .replies
                                .iter()
                                .find(|r| r.id == parent_id)
                                .and_then(|r| r.parent_post_id);
                        }
                        false
                    })
                    .cloned()
                    .collect();

                // Check if root is expanded
                let root_is_expanded = detail_state
                    .modal_expanded_posts
                    .get(&root.id)
                    .copied()
                    .unwrap_or(false);

                if !root_is_expanded || modal_replies.is_empty() {
                    // Only root post is visible (index 0)
                    return 0;
                }

                // Build children map and count visible items
                use std::collections::HashMap;
                let mut children_map: HashMap<Uuid, Vec<&Post>> = HashMap::new();
                for reply in &modal_replies {
                    if let Some(parent_id) = reply.parent_post_id {
                        children_map
                            .entry(parent_id)
                            .or_default()
                            .push(reply);
                    }
                }

                // Count visible flattened items
                let mut visible_count = 0;
                fn count_visible(
                    post_id: &Uuid,
                    children_map: &HashMap<Uuid, Vec<&Post>>,
                    expanded_posts: &std::collections::HashMap<Uuid, bool>,
                    count: &mut usize,
                ) {
                    if let Some(children) = children_map.get(post_id) {
                        for child in children {
                            *count += 1;
                            if expanded_posts.get(&child.id).copied().unwrap_or(false) {
                                count_visible(&child.id, children_map, expanded_posts, count);
                            }
                        }
                    }
                }

                count_visible(
                    &root.id,
                    &children_map,
                    &detail_state.modal_expanded_posts,
                    &mut visible_count,
                );

                // Max index is visible_count (root is 0, replies are 1..=visible_count)
                return visible_count;
            }
        }
        0
    }

    /// Toggle expansion in modal
    pub fn modal_toggle_expansion(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            if detail_state.show_full_post_modal {
                let selected_idx = detail_state.modal_list_state.selected().unwrap_or(0);

                // Index 0 is the root post - toggle it to show/hide direct children
                if selected_idx == 0 {
                    if let Some(modal_post_id) = detail_state.full_post_modal_id {
                        let is_expanded = detail_state
                            .modal_expanded_posts
                            .get(&modal_post_id)
                            .copied()
                            .unwrap_or(false);
                        detail_state
                            .modal_expanded_posts
                            .insert(modal_post_id, !is_expanded);
                    }
                    return;
                }

                // Find the modal root post and build the flattened tree
                if let Some(modal_post_id) = detail_state.full_post_modal_id {
                    let modal_root = if let Some(post) = &detail_state.post {
                        if post.id == modal_post_id {
                            Some(post.clone())
                        } else {
                            detail_state
                                .replies
                                .iter()
                                .find(|r| r.id == modal_post_id)
                                .cloned()
                        }
                    } else {
                        None
                    };

                    if let Some(root) = modal_root {
                        // Filter replies to descendants (excluding the root itself)
                        let modal_replies: Vec<Post> = detail_state
                            .replies
                            .iter()
                            .filter(|reply| {
                                if reply.id == root.id {
                                    return false; // Exclude the root itself
                                }
                                let mut current_parent = reply.parent_post_id;
                                while let Some(parent_id) = current_parent {
                                    if parent_id == root.id {
                                        return true;
                                    }
                                    current_parent = detail_state
                                        .replies
                                        .iter()
                                        .find(|r| r.id == parent_id)
                                        .and_then(|r| r.parent_post_id);
                                }
                                false
                            })
                            .cloned()
                            .collect();

                        // Build tree and flatten to map index to post
                        use std::collections::HashMap;
                        let mut children_map: HashMap<Uuid, Vec<&Post>> = HashMap::new();
                        for reply in &modal_replies {
                            if let Some(parent_id) = reply.parent_post_id {
                                children_map
                                    .entry(parent_id)
                                    .or_default()
                                    .push(reply);
                            }
                        }

                        // Flatten the tree respecting current expansion state
                        let mut flattened_posts = vec![];
                        fn collect_visible(
                            post_id: &Uuid,
                            children_map: &HashMap<Uuid, Vec<&Post>>,
                            expanded_posts: &HashMap<Uuid, bool>,
                            result: &mut Vec<Uuid>,
                        ) {
                            if let Some(children) = children_map.get(post_id) {
                                for child in children {
                                    result.push(child.id);
                                    // Only recurse if this post is expanded
                                    if expanded_posts.get(&child.id).copied().unwrap_or(false) {
                                        collect_visible(
                                            &child.id,
                                            children_map,
                                            expanded_posts,
                                            result,
                                        );
                                    }
                                }
                            }
                        }

                        collect_visible(
                            &root.id,
                            &children_map,
                            &detail_state.modal_expanded_posts,
                            &mut flattened_posts,
                        );

                        // Find the post at selected_idx (accounting for root at index 0)
                        if selected_idx > 0 && selected_idx <= flattened_posts.len() {
                            let post_id = flattened_posts[selected_idx - 1];
                            let is_expanded = detail_state
                                .modal_expanded_posts
                                .get(&post_id)
                                .copied()
                                .unwrap_or(false);
                            detail_state
                                .modal_expanded_posts
                                .insert(post_id, !is_expanded);
                        }
                    }
                }
            }
        }
    }

    /// Select a specific reply in the modal by its ID
    pub fn select_reply_in_modal(&mut self, reply_id: Uuid) {
        if let Some(detail_state) = &mut self.post_detail_state {
            if !detail_state.show_full_post_modal {
                return;
            }

            // Get the modal root post
            let modal_root_id = match detail_state.full_post_modal_id {
                Some(id) => id,
                None => return,
            };

            // Check if the reply_id is the root itself
            if reply_id == modal_root_id {
                detail_state.modal_list_state.select(Some(0));
                return;
            }

            // Find the modal root post
            let modal_root = if let Some(post) = &detail_state.post {
                if post.id == modal_root_id {
                    Some(post.clone())
                } else {
                    detail_state
                        .replies
                        .iter()
                        .find(|r| r.id == modal_root_id)
                        .cloned()
                }
            } else {
                None
            };

            let root = match modal_root {
                Some(r) => r,
                None => return,
            };

            // Filter replies to descendants of modal root
            let modal_replies: Vec<Post> = detail_state
                .replies
                .iter()
                .filter(|reply| {
                    if reply.id == root.id {
                        return false;
                    }
                    let mut current_parent = reply.parent_post_id;
                    while let Some(parent_id) = current_parent {
                        if parent_id == root.id {
                            return true;
                        }
                        current_parent = detail_state
                            .replies
                            .iter()
                            .find(|r| r.id == parent_id)
                            .and_then(|r| r.parent_post_id);
                    }
                    false
                })
                .cloned()
                .collect();

            // Build flattened tree to find the index
            use std::collections::HashMap;
            let mut children_map: HashMap<Uuid, Vec<&Post>> = HashMap::new();
            for reply in &modal_replies {
                if let Some(parent_id) = reply.parent_post_id {
                    children_map
                        .entry(parent_id)
                        .or_default()
                        .push(reply);
                }
            }

            // Expand all ancestors of the target reply so it's visible
            let mut ancestors = vec![];
            let mut current_parent = modal_replies
                .iter()
                .find(|r| r.id == reply_id)
                .and_then(|r| r.parent_post_id);
            
            while let Some(parent_id) = current_parent {
                ancestors.push(parent_id);
                current_parent = modal_replies
                    .iter()
                    .find(|r| r.id == parent_id)
                    .and_then(|r| r.parent_post_id);
            }
            
            // Expand root and all ancestors
            detail_state.modal_expanded_posts.insert(root.id, true);
            for ancestor_id in ancestors {
                detail_state.modal_expanded_posts.insert(ancestor_id, true);
            }

            // Flatten the tree with current expansion state
            let mut flattened_posts = vec![];
            fn collect_visible(
                post_id: &Uuid,
                children_map: &HashMap<Uuid, Vec<&Post>>,
                expanded_posts: &HashMap<Uuid, bool>,
                result: &mut Vec<Uuid>,
            ) {
                if let Some(children) = children_map.get(post_id) {
                    for child in children {
                        result.push(child.id);
                        if expanded_posts.get(&child.id).copied().unwrap_or(false) {
                            collect_visible(&child.id, children_map, expanded_posts, result);
                        }
                    }
                }
            }

            collect_visible(
                &root.id,
                &children_map,
                &detail_state.modal_expanded_posts,
                &mut flattened_posts,
            );

            // Find the index of the target reply (add 1 because root is at index 0)
            if let Some(pos) = flattened_posts.iter().position(|&id| id == reply_id) {
                detail_state.modal_list_state.select(Some(pos + 1));
            }
        }
    }

    /// Toggle follow/unfollow for user in profile view
    pub fn toggle_follow_in_profile_view(&mut self) {
        if let Some(profile) = &mut self.user_profile_view {
            match &profile.relationship {
                RelationshipStatus::Following | RelationshipStatus::MutualFriends => {
                    // Will unfollow (handled async in main loop)
                }
                RelationshipStatus::None | RelationshipStatus::FollowsYou => {
                    // Will follow (handled async in main loop)
                }
                RelationshipStatus::Self_ => {
                    // Cannot follow yourself
                }
            }
        }
    }

    /// Handle keyboard events for user profile view
    pub fn handle_user_profile_view_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close_user_profile_view();
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Toggle follow/unfollow (will be handled async in main loop)
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                // Open DM if mutual friends (will be handled async in main loop)
            }
            _ => {}
        }
        Ok(())
    }

    /// Load user profile view
    pub async fn load_user_profile_view(&mut self, user_id: String) -> Result<()> {
        match self.api_client.get_user_profile_view(user_id.clone()).await {
            Ok(profile_data) => {
                self.user_profile_view = Some(UserProfileViewState {
                    user_id: profile_data.id,
                    username: profile_data.username,
                    bio: profile_data.bio,
                    join_date: profile_data.join_date,
                    follower_count: profile_data.follower_count,
                    following_count: profile_data.following_count,
                    post_count: profile_data.post_count,
                    relationship: match profile_data.relationship {
                        fido_types::RelationshipStatus::Self_ => RelationshipStatus::Self_,
                        fido_types::RelationshipStatus::MutualFriends => {
                            RelationshipStatus::MutualFriends
                        }
                        fido_types::RelationshipStatus::Following => RelationshipStatus::Following,
                        fido_types::RelationshipStatus::FollowsYou => {
                            RelationshipStatus::FollowsYou
                        }
                        fido_types::RelationshipStatus::None => RelationshipStatus::None,
                    },
                    loading: false,
                    error: None,
                });
            }
            Err(e) => {
                // Show error but don't open profile view
                self.posts_state.error = Some(format!("Failed to load profile: {}", e));
            }
        }
        Ok(())
    }

    /// Follow user in profile view
    pub async fn follow_user_in_profile_view(&mut self, user_id: String) -> Result<()> {
        match self.api_client.follow_user(user_id.clone()).await {
            Ok(_) => {
                // Reload profile to get updated relationship status
                self.load_user_profile_view(user_id).await?;
            }
            Err(e) => {
                if let Some(profile) = &mut self.user_profile_view {
                    profile.error = Some(format!("Failed to follow: {}", e));
                }
            }
        }
        Ok(())
    }

    /// Unfollow user in profile view
    pub async fn unfollow_user_in_profile_view(&mut self, user_id: String) -> Result<()> {
        match self.api_client.unfollow_user(user_id.clone()).await {
            Ok(_) => {
                // Reload profile to get updated relationship status
                self.load_user_profile_view(user_id).await?;
            }
            Err(e) => {
                if let Some(profile) = &mut self.user_profile_view {
                    profile.error = Some(format!("Failed to unfollow: {}", e));
                }
            }
        }
        Ok(())
    }



    pub fn show_delete_confirmation(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            let current_user_id = self.auth_state.current_user.as_ref().map(|u| u.id);

            // Use existing helper method to get the post that would be deleted
            if let Some(deletable_post) = detail_state.get_deletable_post() {
                // Only show confirmation if user owns the post
                if current_user_id == Some(deletable_post.author_id) {
                    detail_state.show_delete_confirmation = true;
                }
            }
        }
    }

    pub fn cancel_delete_confirmation(&mut self) {
        if let Some(detail_state) = &mut self.post_detail_state {
            detail_state.show_delete_confirmation = false;
        }
    }

    pub async fn delete_post(&mut self) -> Result<()> {
        // Extract the data we need before borrowing mutably
        let (post_id, is_reply, main_post_id) = {
            let detail_state = match &self.post_detail_state {
                Some(state) => state,
                None => return Ok(()),
            };

            // Use the existing helper method to determine what to delete
            let deletable_post = match detail_state.get_deletable_post() {
                Some(post) => post,
                None => return Ok(()), // No post to delete
            };

            let post_id = deletable_post.id;
            let is_reply = deletable_post.parent_post_id.is_some();
            let main_post_id = detail_state.post.as_ref().map(|p| p.id);

            (post_id, is_reply, main_post_id)
        };

        // Clear error before API call
        if let Some(detail_state) = &mut self.post_detail_state {
            detail_state.error = None;
        }

        match self.api_client.delete_post(post_id).await {
            Ok(_) => {
                if is_reply {
                    // Deleted a reply - reload the post detail to refresh replies
                    if let Some(main_id) = main_post_id {
                        if let Some(detail_state) = &mut self.post_detail_state {
                            detail_state.show_delete_confirmation = false;
                        }
                        self.load_post_detail(main_id).await?;
                        if let Some(detail_state) = &mut self.post_detail_state {
                            detail_state.message = Some(("✓ Reply deleted successfully".to_string(), std::time::Instant::now()));
                        }
                    }
                } else {
                    // Deleted the main post - close detail view and remove from feed
                    self.close_post_detail();
                    if let Some(index) = self.posts_state.posts.iter().position(|p| p.id == post_id)
                    {
                        self.posts_state.posts.remove(index);
                        if self.posts_state.posts.is_empty() {
                            self.posts_state.list_state.select(None);
                        } else if index >= self.posts_state.posts.len() {
                            self.posts_state
                                .list_state
                                .select(Some(self.posts_state.posts.len() - 1));
                        }
                    }
                    self.posts_state.message = Some(("✓ Post deleted successfully".to_string(), std::time::Instant::now()));
                }
            }
            Err(e) => {
                if let Some(detail_state) = &mut self.post_detail_state {
                    detail_state.error = Some(categorize_error(&e.to_string()));
                    detail_state.show_delete_confirmation = false;
                }
            }
        }
        Ok(())
    }

    pub fn handle_delete_confirmation_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Confirmation handled in main.rs async
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.cancel_delete_confirmation();
            }
            _ => {}
        }
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a reply is a descendant of a given post
fn is_descendant_of_post(reply: &Post, ancestor_id: &Uuid, all_replies: &[Post]) -> bool {
    let mut current_parent = reply.parent_post_id;
    
    while let Some(parent_id) = current_parent {
        if parent_id == *ancestor_id {
            return true;
        }
        current_parent = all_replies.iter()
            .find(|r| r.id == parent_id)
            .and_then(|r| r.parent_post_id);
    }
    
    false
}

/// Categorize error messages for better user feedback
fn categorize_error(error_str: &str) -> String {
    let error_lower = error_str.to_lowercase();

    // Network errors
    if error_lower.contains("connection")
        || error_lower.contains("timeout")
        || error_lower.contains("network")
    {
        let modifier = get_modifier_key_name();
        return format!("Network Error: Connection failed. Check your network and try again (Press {}+R to retry)", modifier);
    }

    // Authorization errors
    if error_lower.contains("401")
        || error_lower.contains("403")
        || error_lower.contains("unauthorized")
        || error_lower.contains("forbidden")
    {
        return "Authorization Error: Session expired or insufficient permissions. Please log in again (Press Shift+L to logout)".to_string();
    }

    // Validation errors
    if error_lower.contains("400")
        || error_lower.contains("validation")
        || error_lower.contains("invalid")
    {
        return format!("Validation Error: {}", error_str);
    }

    // Server errors
    if error_lower.contains("500") || error_lower.contains("502") || error_lower.contains("503") {
        let modifier = get_modifier_key_name();
        return format!("Server Error: The server is experiencing issues. Please try again later (Press {}+R to retry)", modifier);
    }

    // Generic error with retry instruction
    let modifier = get_modifier_key_name();
    format!("Error: {} (Press {}+R to retry)", error_str, modifier)
}

#[cfg(test)]
mod tests;
