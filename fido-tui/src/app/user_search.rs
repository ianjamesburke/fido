use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use super::{App, InputMode, UserSearchResult};

impl App {
    /// Handle user search modal key events
    pub fn handle_user_search_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Down => {
                self.user_search_navigate(1);
            }
            KeyCode::Up => {
                self.user_search_navigate(-1);
            }
            KeyCode::Backspace => {
                self.user_search_state.search_query.pop();
                // Search will be triggered in main loop
            }
            KeyCode::Enter => {
                // View profile of selected user (will be handled in main loop)
            }
            KeyCode::Char(c) => {
                // Handle navigation keys
                match c {
                    'j' | 'J' => self.user_search_navigate(1),
                    'k' | 'K' => self.user_search_navigate(-1),
                    'd' | 'D' => {
                        // Start DM with selected user (will be handled in main loop)
                    }
                    _ => {
                        // Regular character input for search
                        self.user_search_state.search_query.push(c);
                        // Search will be triggered in main loop
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Open user search modal
    pub fn open_user_search_modal(&mut self) {
        self.user_search_state.show_modal = true;
        self.user_search_state.search_query.clear();
        self.user_search_state.search_results.clear();
        self.user_search_state.selected_index = 0;
        self.user_search_state.error = None;
        self.input_mode = InputMode::Typing;
    }

    /// Close user search modal
    pub fn close_user_search_modal(&mut self) {
        self.user_search_state.show_modal = false;
        self.user_search_state.search_query.clear();
        self.user_search_state.search_results.clear();
        self.user_search_state.selected_index = 0;
        self.input_mode = InputMode::Navigation;
    }

    /// Search users by query
    pub async fn search_users(&mut self) -> Result<()> {
        let query = self.user_search_state.search_query.clone();

        // Require at least 2 characters
        if query.len() < 2 {
            self.user_search_state.search_results.clear();
            return Ok(());
        }

        self.user_search_state.loading = true;
        self.user_search_state.error = None;

        match self.api_client.search_users(query).await {
            Ok(results) => {
                self.user_search_state.search_results = results
                    .into_iter()
                    .map(|r| UserSearchResult {
                        username: r.username,
                    })
                    .collect();
                self.user_search_state.selected_index = 0;
                self.user_search_state.loading = false;
            }
            Err(e) => {
                self.user_search_state.error = Some(format!("Search failed: {}", e));
                self.user_search_state.loading = false;
            }
        }

        Ok(())
    }

    /// Navigate user search results
    pub fn user_search_navigate(&mut self, direction: i32) {
        if self.user_search_state.search_results.is_empty() {
            return;
        }

        let len = self.user_search_state.search_results.len();
        let current = self.user_search_state.selected_index;

        self.user_search_state.selected_index = if direction > 0 {
            (current + 1).min(len.saturating_sub(1))
        } else {
            current.saturating_sub(1)
        };
    }
}
