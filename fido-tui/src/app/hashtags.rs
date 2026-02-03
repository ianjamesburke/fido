use super::*;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
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
}
