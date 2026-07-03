use anyhow::Result;

use super::{categorize_error, App, FilterTab, InputMode, PostFilter};
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    pub fn queue_vote_on_post(
        &mut self,
        post_id: uuid::Uuid,
        direction: crate::api::VoteDirection,
    ) {
        let api_client = self.api_client.clone();
        let handle =
            tokio::spawn(
                async move { (post_id, api_client.vote_on_post(post_id, direction).await) },
            );
        self.pending_vote_tasks.push(handle);
    }

    pub async fn flush_finished_vote_tasks(&mut self) {
        let mut i = 0;
        while i < self.pending_vote_tasks.len() {
            if !self.pending_vote_tasks[i].is_finished() {
                i += 1;
                continue;
            }

            let handle = self.pending_vote_tasks.swap_remove(i);
            match handle.await {
                Ok((_, Ok(_))) => {}
                Ok((_, Err(e))) => {
                    self.posts_state.error = Some(categorize_error(&e.to_string()));
                }
                Err(e) => {
                    self.posts_state.error = Some(format!("Vote update failed: {}", e));
                }
            }
        }
    }

    /// Load posts from API
    pub async fn load_posts(&mut self) -> Result<()> {
        self.posts_state.loading = true;
        self.posts_state.error = None;

        // Yield to allow UI to render the loading state
        tokio::task::yield_now().await;

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

        let Some(community_id) = self.community.as_ref().map(|c| c.id) else {
            self.posts_state.posts.clear();
            self.posts_state.list_state.select(None);
            self.posts_state.loading = false;
            return Ok(());
        };

        self.posts_state.message = None;

        // Apply current filter
        let result = match &self.posts_state.current_filter {
            PostFilter::All => {
                self.api_client
                    .get_posts(
                        community_id,
                        Some(max_posts),
                        Some(sort_order.clone()),
                        None,
                        None,
                    )
                    .await
            }
            PostFilter::Hashtag(tag) => {
                self.api_client
                    .get_posts(
                        community_id,
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
                        community_id,
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
                            community_id,
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
                            community_id,
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
                self.posts_state.rebuild_feed();
                // Server now includes user_vote in each post
                if has_posts {
                    self.posts_state.list_state.select(Some(0));
                } else {
                    self.posts_state.list_state.select(None);
                }
                self.posts_state.loading = false;
                self.posts_state.activity_loading = true;
                self.posts_state.activity_pending_load = true;
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
        if let Some(list_index) = self.posts_state.list_state.selected() {
            let Some(post_index) = self.posts_state.list_index_to_post_index(list_index) else {
                // Selection is an activity row, not a post - nothing to vote on
                return Ok(());
            };

            // Clear any previous errors
            self.posts_state.error = None;

            let selected_post = &mut self.posts_state.posts[post_index];
            let post_id = selected_post.id;

            // Check if user has already voted on this post
            let previous_vote = selected_post.user_vote.clone();

            // If user is trying to vote the same direction again, silently ignore it
            if let Some(ref prev_direction) = previous_vote {
                if prev_direction == direction {
                    return Ok(());
                }
            }

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
            self.queue_vote_on_post(post_id, vote_direction);
        }
        Ok(())
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
        // Load both lists concurrently to keep filter modal snappy.
        let hashtags_future = self.api_client.get_followed_hashtags();
        let following_future = self.api_client.get_following_list();
        let (hashtags_result, following_result) = tokio::join!(hashtags_future, following_future);

        match hashtags_result {
            Ok(hashtags) => {
                self.posts_state.filter_modal_state.hashtag_list = hashtags;
            }
            Err(_) => {
                // Silently fail, just show empty list
                self.posts_state.filter_modal_state.hashtag_list.clear();
            }
        }

        match following_result {
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

    pub fn next_post(&mut self) {
        if self.posts_state.feed_entries.is_empty() {
            return;
        }

        let offset = self.posts_state.items_before_posts();
        let total_items = offset + self.posts_state.feed_entries.len();
        let current = self.posts_state.list_state.selected();

        let next = match current {
            Some(i) => {
                // Stop at bottom, don't wrap around
                if i >= total_items - 1 {
                    // At the end of the feed - show "End of Feed" indicator
                    self.posts_state.at_end_of_feed = true;
                    i
                } else {
                    self.posts_state.at_end_of_feed = false;
                    i + 1
                }
            }
            None => {
                self.posts_state.at_end_of_feed = false;
                offset
            }
        };

        self.posts_state.list_state.select(Some(next));
    }

    pub fn previous_post(&mut self) {
        if self.posts_state.feed_entries.is_empty() {
            return;
        }

        // Clear end-of-feed indicator when scrolling up
        self.posts_state.at_end_of_feed = false;

        let offset = self.posts_state.items_before_posts();
        let current = self.posts_state.list_state.selected();

        match current {
            Some(i) if i > offset => {
                self.posts_state.list_state.select(Some(i - 1));
            }
            _ => {
                // Already at top or no selection
                self.posts_state.list_state.select(Some(offset));
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
