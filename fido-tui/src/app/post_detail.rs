use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use fido_types::Post;
use ratatui::widgets::ListState;
use uuid::Uuid;

use super::{categorize_error, App, InputMode, PostDetailState};

impl App {
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
            show_full_post_modal: true,        // Open modal directly
            full_post_modal_id: Some(post_id), // Set the post ID for modal
            modal_list_state,
            modal_expanded_posts, // Root post pre-expanded
        });
        self.viewing_post_detail = true;
        self.load_post_detail(post_id).await?;
        Ok(())
    }

    pub(crate) async fn load_post_detail(&mut self, post_id: Uuid) -> Result<()> {
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
        let vote_direction = crate::api::VoteDirection::from_str(direction)
            .ok_or_else(|| anyhow::anyhow!("Invalid vote direction: {}", direction))?;
        if self.is_demo_mode {
            match self.api_client.vote_on_post(post_id, vote_direction).await {
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
        } else {
            self.queue_vote_on_post(post_id, vote_direction);
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
                        children_map.entry(parent_id).or_default().push(reply);
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
        let vote_direction = crate::api::VoteDirection::from_str(direction)
            .ok_or_else(|| anyhow::anyhow!("Invalid vote direction: {}", direction))?;
        if self.is_demo_mode {
            match self.api_client.vote_on_post(post_id, vote_direction).await {
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
        } else {
            self.queue_vote_on_post(post_id, vote_direction);
        }

        Ok(())
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

    /// Close user profile view
    pub fn close_user_profile_view(&mut self) {
        self.user_profile_view = None;

        // Reopen social modal if flag is set
        if self.friends_state.return_to_modal_after_profile {
            self.friends_state.show_friends_modal = true;
            self.friends_state.return_to_modal_after_profile = false;
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
                        children_map.entry(parent_id).or_default().push(reply);
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
                                children_map.entry(parent_id).or_default().push(reply);
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
                    children_map.entry(parent_id).or_default().push(reply);
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
                            detail_state.message = Some((
                                "✓ Reply deleted successfully".to_string(),
                                std::time::Instant::now(),
                            ));
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
                    self.posts_state.message = Some((
                        "✓ Post deleted successfully".to_string(),
                        std::time::Instant::now(),
                    ));
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
