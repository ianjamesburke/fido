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
        self.posts_state.current_filter = PostFilter::All;

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
            self.posts_state.rebuild_feed();
            return Ok(());
        };

        self.posts_state.message = None;

        let result = self
            .api_client
            .get_posts(
                community_id,
                Some(max_posts),
                Some(sort_order.clone()),
                None,
                None,
            )
            .await;

        match result {
            Ok(posts) => {
                let has_posts = !posts.is_empty();
                self.posts_state.posts = posts;
                self.realtime_state
                    .seen_posts
                    .extend(self.posts_state.posts.iter().map(|post| post.id));
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

    pub fn is_current_community_admin(&self) -> bool {
        self.community.as_ref().and_then(|community| community.role)
            == Some(fido_types::MembershipRole::Admin)
    }

    pub fn clear_approval_queue(&mut self) {
        self.posts_state.pending_threads.clear();
        self.posts_state.pending_threads_list_state.select(None);
        self.posts_state.show_approval_queue = false;
        self.posts_state.pending_threads_loading = false;
        self.posts_state.pending_threads_loaded = false;
        self.posts_state.pending_threads_error = None;
    }

    pub async fn open_approval_queue(&mut self) -> Result<()> {
        if !self.is_current_community_admin() {
            self.posts_state.error =
                Some("Community admin role required to review pending threads.".to_string());
            return Ok(());
        }
        self.posts_state.show_approval_queue = true;
        self.posts_state.pending_threads_error = None;
        self.load_pending_threads().await
    }

    pub fn close_approval_queue(&mut self) {
        self.posts_state.show_approval_queue = false;
        self.posts_state.pending_threads_error = None;
        self.input_mode = InputMode::Navigation;
    }

    pub async fn load_pending_threads(&mut self) -> Result<()> {
        let Some(community_id) = self.current_community_id() else {
            self.posts_state.pending_threads_error =
                Some("Open a community before reviewing pending threads.".to_string());
            return Ok(());
        };

        self.posts_state.pending_threads_loading = true;
        self.posts_state.pending_threads_error = None;

        match self
            .api_client
            .get_pending_posts(community_id, Some(50))
            .await
        {
            Ok(posts) => {
                let has_posts = !posts.is_empty();
                self.posts_state.pending_threads = posts;
                self.posts_state.pending_threads_loaded = true;
                if has_posts {
                    self.posts_state.pending_threads_list_state.select(Some(0));
                } else {
                    self.posts_state.pending_threads_list_state.select(None);
                }
            }
            Err(e) => {
                self.posts_state.pending_threads_error = Some(categorize_error(&e.to_string()));
            }
        }

        self.posts_state.pending_threads_loading = false;
        Ok(())
    }

    pub fn next_pending_thread(&mut self) {
        let len = self.posts_state.pending_threads.len();
        if len == 0 {
            self.posts_state.pending_threads_list_state.select(None);
            return;
        }
        let current = self
            .posts_state
            .pending_threads_list_state
            .selected()
            .unwrap_or(0);
        self.posts_state
            .pending_threads_list_state
            .select(Some((current + 1).min(len - 1)));
    }

    pub fn previous_pending_thread(&mut self) {
        let len = self.posts_state.pending_threads.len();
        if len == 0 {
            self.posts_state.pending_threads_list_state.select(None);
            return;
        }
        let current = self
            .posts_state
            .pending_threads_list_state
            .selected()
            .unwrap_or(0);
        self.posts_state
            .pending_threads_list_state
            .select(Some(current.saturating_sub(1)));
    }

    fn selected_pending_thread_id(&self) -> Option<uuid::Uuid> {
        self.posts_state
            .pending_threads_list_state
            .selected()
            .and_then(|i| self.posts_state.pending_threads.get(i))
            .map(|post| post.id)
    }

    pub async fn approve_selected_pending_thread(&mut self) -> Result<()> {
        let Some(post_id) = self.selected_pending_thread_id() else {
            return Ok(());
        };

        match self.api_client.approve_post(post_id).await {
            Ok(post) => {
                self.remove_pending_thread(post_id);
                self.load_posts().await?;
                self.posts_state.message = Some((
                    format!("Approved @{}'s thread.", post.author_username),
                    std::time::Instant::now(),
                ));
            }
            Err(e) => {
                self.posts_state.pending_threads_error = Some(categorize_error(&e.to_string()));
            }
        }
        Ok(())
    }

    pub async fn reject_selected_pending_thread(&mut self) -> Result<()> {
        let Some(post_id) = self.selected_pending_thread_id() else {
            return Ok(());
        };

        match self.api_client.reject_post(post_id).await {
            Ok(post) => {
                self.remove_pending_thread(post_id);
                self.posts_state.message = Some((
                    format!("Rejected @{}'s thread.", post.author_username),
                    std::time::Instant::now(),
                ));
            }
            Err(e) => {
                self.posts_state.pending_threads_error = Some(categorize_error(&e.to_string()));
            }
        }
        Ok(())
    }

    fn remove_pending_thread(&mut self, post_id: uuid::Uuid) {
        let previous = self
            .posts_state
            .pending_threads_list_state
            .selected()
            .unwrap_or(0);
        self.posts_state
            .pending_threads
            .retain(|post| post.id != post_id);
        if self.posts_state.pending_threads.is_empty() {
            self.posts_state.pending_threads_list_state.select(None);
        } else {
            self.posts_state.pending_threads_list_state.select(Some(
                previous.min(self.posts_state.pending_threads.len().saturating_sub(1)),
            ));
        }
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
    #[allow(dead_code)]
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
    fn save_filter_preference(&self) {}

    /// Load filter preference from disk
    pub fn load_filter_preference(&mut self) {
        self.posts_state.current_filter = PostFilter::All;
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
