use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use super::{categorize_error, App, InputMode};

impl App {
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
                .get_posts(Some(100), Some("Newest".to_string()), None, None)
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
                }
                Err(e) => {
                    let error_msg = categorize_error(&e.to_string());
                    self.profile_state.error =
                        Some(format!("{} (Switch tabs to retry)", error_msg));
                }
            }

            self.profile_state.loading = false;
        }

        Ok(())
    }

    /// Open bio edit modal
    pub fn open_edit_bio_modal(&mut self) {
        if let Some(profile) = &self.profile_state.profile {
            self.profile_state.show_edit_bio_modal = true;
            self.profile_state.edit_bio_content = profile.bio.clone().unwrap_or_default();
            self.profile_state.edit_bio_cursor_position =
                self.profile_state.edit_bio_content.chars().count();
            self.input_mode = InputMode::Typing;
        }
    }

    /// Close bio edit modal
    pub fn close_edit_bio_modal(&mut self) {
        self.profile_state.show_edit_bio_modal = false;
        self.profile_state.edit_bio_content.clear();
        self.profile_state.edit_bio_cursor_position = 0;
        self.input_mode = InputMode::Navigation;
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

}
