use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Style;
use tui_textarea::TextArea;
use uuid::Uuid;

use super::{categorize_error, App, Conversation, DMSelection, DmRequest, InputMode, UserInfo};

impl App {
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

    /// Load DM conversations
    pub async fn load_conversations(&mut self) -> Result<()> {
        self.dms_state.loading = true;
        self.dms_state.error = None;

        self.dms_state.pending_requests = self
            .api_client
            .get_pending_dm_requests()
            .await
            .map(|reqs| {
                reqs.into_iter()
                    .filter_map(|r| {
                        Some(DmRequest {
                            from_user_id: r.from_user_id.parse().ok()?,
                            from_username: r.from_username,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        match self.api_client.get_conversations().await {
            Ok(convos) => {
                self.dms_state.conversations = convos
                    .into_iter()
                    .filter_map(|c| {
                        Some(Conversation {
                            other_user_id: c.other_user_id.parse().ok()?,
                            other_username: c.other_username,
                            last_message: c.last_message,
                            unread_count: c.unread_count as i32,
                            state: c.state,
                            initiated_by_me: c.initiated_by_me,
                        })
                    })
                    .collect();

                // Incoming pending conversations are represented by the requests list;
                // filter them out of the conversation list to avoid double entries.
                self.dms_state.conversations.retain(|c| {
                    c.state != fido_types::DmConversationState::Pending || c.initiated_by_me
                });

                // Update unread_counts HashMap from conversations
                self.dms_state.unread_counts.clear();
                for convo in &self.dms_state.conversations {
                    self.dms_state
                        .unread_counts
                        .insert(convo.other_user_id, convo.unread_count as usize);
                }

                // Select first conversation if available, otherwise stay on NewConversation
                if !self.dms_state.conversations.is_empty() {
                    self.dms_state.selection = DMSelection::Conversation(0);
                    self.dms_state.messages.clear();
                    self.dms_state.current_conversation_user = None;
                    self.dms_state.needs_message_load = false;
                } else {
                    self.dms_state.selection = DMSelection::NewConversation;
                    self.dms_state.messages.clear();
                    self.dms_state.current_conversation_user = None;
                }

                self.dms_state.conversations_loaded = true;
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

        // Check if a conversation is selected (not the "New Conversation" button or pending draft)
        let selected_index = match &self.dms_state.selection {
            DMSelection::NewConversation => return Ok(()), // Nothing to load
            DMSelection::PendingDraft => return Ok(()),    // Nothing to load yet
            DMSelection::Request(_) => return Ok(()),      // Accept the request first
            DMSelection::Conversation(index) => *index,
        };
        if selected_index >= self.dms_state.conversations.len() {
            return Ok(());
        }

        let conversation = &self.dms_state.conversations[selected_index];
        let other_user_id = conversation.other_user_id;

        match self.api_client.get_conversation(other_user_id).await {
            Ok(messages) => {
                self.dms_state.messages = messages;
                self.realtime_state
                    .seen_dm_messages
                    .extend(self.dms_state.messages.iter().map(|message| message.id));

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
    pub async fn mark_conversation_as_read(&mut self, user_id: Uuid) -> Result<()> {
        // Set current conversation user
        self.dms_state.current_conversation_user = Some(user_id);

        // Opening a conversation marks it as read server-side in GET /dms/conversations/:user_id.
        // Keep local read state in sync without issuing another network round-trip.
        self.dms_state.unread_counts.insert(user_id, 0);

        for msg in self.dms_state.messages.iter_mut() {
            msg.is_read = true;
        }

        if let Some(convo) = self
            .dms_state
            .conversations
            .iter_mut()
            .find(|c| c.other_user_id == user_id)
        {
            convo.unread_count = 0;
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
        let to_username =
            if let Some(pending_username) = &self.dms_state.pending_conversation_username {
                pending_username.clone()
            } else {
                // Regular conversation - get from selected
                let selected_index = match &self.dms_state.selection {
                    DMSelection::NewConversation => {
                        self.dms_state.error =
                            Some("Press Enter or N to start a new conversation.".to_string());
                        return Ok(());
                    }
                    DMSelection::PendingDraft => {
                        // This shouldn't happen since we check pending_conversation_username above
                        self.dms_state.error = Some("No recipient selected.".to_string());
                        return Ok(());
                    }
                    DMSelection::Request(_) => {
                        self.dms_state.error =
                            Some("Accept the request first before sending a message.".to_string());
                        return Ok(());
                    }
                    DMSelection::Conversation(index) => *index,
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
                        self.dms_state.selection = DMSelection::Conversation(0);
                        self.dms_state.needs_message_load = true;
                    }
                } else {
                    // Regular message in existing conversation
                    self.load_conversation_messages().await?;

                    // Keep unread count at 0 for current conversation
                    if let Some(index) = self.dms_state.selection.conversation_index() {
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
                self.dms_state.available_mutual_friends = friends
                    .into_iter()
                    .filter_map(|f| {
                        Some(UserInfo {
                            id: f.id.parse().ok()?,
                            username: f.username,
                            follower_count: f.follower_count,
                            following_count: f.following_count,
                        })
                    })
                    .collect();
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
        self.dms_state.new_conversation_selected_index = 0;
        self.dms_state.new_conversation_search_mode = false;
        self.dms_state.new_conversation_search_query.clear();
        self.input_mode = InputMode::Navigation;
    }

    /// Get filtered mutual friends list for new conversation modal
    pub fn get_filtered_mutual_friends(&self) -> Vec<&UserInfo> {
        let query = self.dms_state.new_conversation_search_query.to_lowercase();

        if query.is_empty() {
            self.dms_state.available_mutual_friends.iter().collect()
        } else {
            self.dms_state
                .available_mutual_friends
                .iter()
                .filter(|u| u.username.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Start new conversation (just prepare, don't send anything yet)
    pub async fn start_new_conversation(&mut self) -> Result<()> {
        // Get the selected user from the filtered list
        let filtered = self.get_filtered_mutual_friends();
        if filtered.is_empty() {
            return Ok(());
        }

        let selected_index = self
            .dms_state
            .new_conversation_selected_index
            .min(filtered.len() - 1);
        let to_username = filtered[selected_index].username.clone();

        self.dms_state.error = None;

        // Store the username for the pending conversation
        self.dms_state.pending_conversation_username = Some(to_username.clone());

        // Close modal and clear messages (show empty conversation)
        self.close_new_conversation_modal();
        self.dms_state.messages.clear();

        // Set selection to PendingDraft so the pending conversation is selected in the UI
        self.dms_state.selection = DMSelection::PendingDraft;

        // Switch to typing mode so user can immediately start composing
        self.input_mode = InputMode::Typing;

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
            InputMode::Navigation => match key.code {
                // Navigation using the new DMSelection enum methods
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.dms_state.navigate_down();
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.dms_state.navigate_up();
                }
                // 'N' key opens new conversation modal directly (shortcut)
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.dms_state.show_new_conversation_modal = true;
                    self.dms_state.new_conversation_username.clear();
                    self.dms_state.new_conversation_search_query.clear();
                    self.dms_state.new_conversation_selected_index = 0;
                    self.input_mode = InputMode::Typing;
                }
                KeyCode::Enter => match &self.dms_state.selection {
                    DMSelection::NewConversation => {
                        // Open new conversation modal
                        self.dms_state.show_new_conversation_modal = true;
                        self.dms_state.new_conversation_username.clear();
                        self.dms_state.new_conversation_search_query.clear();
                        self.dms_state.new_conversation_selected_index = 0;
                        self.input_mode = InputMode::Typing;
                    }
                    DMSelection::PendingDraft => {
                        // Focus on message input for the pending draft
                        self.input_mode = InputMode::Typing;
                    }
                    DMSelection::Request(_) => {
                        // Nothing to do on Enter — use 'a'/'x' to accept/decline
                    }
                    DMSelection::Conversation(idx) => {
                        // Select this conversation and load messages
                        if *idx < self.dms_state.conversations.len() {
                            self.dms_state.needs_message_load = true;
                            self.input_mode = InputMode::Typing;
                        }
                    }
                },
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    // Profile of selected conversation — handled async in event_loop
                }
                KeyCode::Char('a')
                | KeyCode::Char('A')
                | KeyCode::Char('x')
                | KeyCode::Char('X')
                    if matches!(self.dms_state.selection, DMSelection::Request(_)) =>
                {
                    // accept/decline — handled async in event_loop
                }
                _ => {
                    // A Request row has no compose surface — stray keys must not
                    // enter Typing mode (there's nowhere for them to go).
                    if matches!(self.dms_state.selection, DMSelection::Request(_)) {
                        return Ok(());
                    }
                    // Any other key starts typing mode (for message input)
                    self.input_mode = InputMode::Typing;
                    self.handle_dm_input(key);
                    // Trigger message load if not already loaded for this conversation
                    if self.dms_state.messages.is_empty() {
                        self.dms_state.needs_message_load = true;
                    }
                }
            },
            InputMode::Typing => match key.code {
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
            },
        }
        Ok(())
    }

    /// Handle keys for new conversation modal
    pub fn handle_new_conversation_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        // If in search mode, handle search input
        if self.dms_state.new_conversation_search_mode {
            match key.code {
                KeyCode::Char(c) => {
                    self.dms_state.new_conversation_search_query.push(c);
                    self.dms_state.new_conversation_selected_index = 0;
                }
                KeyCode::Backspace => {
                    self.dms_state.new_conversation_search_query.pop();
                    self.dms_state.new_conversation_selected_index = 0;
                }
                KeyCode::Esc => {
                    self.dms_state.new_conversation_search_mode = false;
                }
                _ => {}
            }
            return Ok(());
        }

        // Normal navigation mode
        match key.code {
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                let filtered = self.get_filtered_mutual_friends();
                if !filtered.is_empty() {
                    self.dms_state.new_conversation_selected_index =
                        (self.dms_state.new_conversation_selected_index + 1)
                            .min(filtered.len() - 1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.dms_state.new_conversation_selected_index = self
                    .dms_state
                    .new_conversation_selected_index
                    .saturating_sub(1);
            }
            KeyCode::Char('/') => {
                self.dms_state.new_conversation_search_mode = true;
            }
            KeyCode::Enter => {
                // Start conversation with selected user (will be handled async in main loop)
            }
            _ => {}
        }
        Ok(())
    }
}
