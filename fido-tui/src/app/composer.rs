use super::{categorize_error, App, ComposerMode, InputMode};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Style;
use tui_textarea::TextArea;
use uuid::Uuid;

impl App {
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
            Style::default().fg(theme.primary), // Use primary color for better visibility
        );
        textarea.set_cursor_style(
            Style::default().fg(theme.background).bg(theme.primary), // Visible cursor
        );
        textarea.set_cursor_line_style(
            Style::default(), // No special cursor line styling
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
                Some(ComposerMode::Reply { .. }) => {
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
                let Some(community_id) = self.current_community_id else {
                    self.posts_state.error = Some("No community selected".to_string());
                    return Ok(());
                };
                match self
                    .api_client
                    .create_post(community_id, parsed_content)
                    .await
                {
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
                let root_post_id = self
                    .post_detail_state
                    .as_ref()
                    .and_then(|s| s.post.as_ref().map(|p| p.id))
                    .unwrap_or(post_id);

                if let Some(detail_state) = &mut self.post_detail_state {
                    detail_state.error = None;
                }
                match self.api_client.create_reply(post_id, parsed_content).await {
                    Ok(new_reply) => {
                        let new_reply_id = new_reply.id;

                        // Optimistic update: increment reply count in cached post
                        if let Some(cached_post) =
                            self.posts_state.posts.iter_mut().find(|p| p.id == post_id)
                        {
                            cached_post.reply_count += 1;
                        }

                        // Ensure we stay in thread view
                        self.viewing_post_detail = true;

                        // Reload the root thread, not the parent post
                        self.load_post_detail(root_post_id).await?;

                        // Explicitly ensure modal is open after reload
                        if let Some(detail_state) = &mut self.post_detail_state {
                            detail_state.show_full_post_modal = true;
                            detail_state.full_post_modal_id = Some(root_post_id);
                        }

                        // Select the newly created reply in the modal
                        self.select_reply_in_modal(new_reply_id);

                        // Close composer AFTER all modal state has been restored
                        self.close_composer();
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
                                "Network Error: Connection failed - check your network and try again"
                                    .to_string()
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
}
