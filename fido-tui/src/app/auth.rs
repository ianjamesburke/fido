use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use super::{App, Screen, Tab};

impl App {
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
}
