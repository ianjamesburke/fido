use anyhow::Result;
use std::collections::HashSet;
use std::time::Duration;

use super::{categorize_error, App, FilterTab, InputMode, PostFilter};
use crossterm::event::{KeyCode, KeyEvent};

impl App {
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
            .unwrap_or_else(|| "Newest".to_string());

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
            let vote_direction = crate::api::VoteDirection::from_str(direction)
                .ok_or_else(|| anyhow::anyhow!("Invalid vote direction: {}", direction))?;
            match self.api_client.vote_on_post(post_id, vote_direction).await {
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
                let mut found_hashtags = HashSet::new();
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

}
