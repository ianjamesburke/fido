use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use super::{categorize_error, App, SettingsField};

impl App {
    /// Load settings
    pub async fn load_settings(&mut self) -> Result<()> {
        self.settings_state.loading = true;
        self.settings_state.error = None;

        match self.api_client.get_config().await {
            Ok(config) => {
                self.settings_state.max_posts_input = config.max_posts_display.to_string();
                self.settings_state.original_max_posts_input = config.max_posts_display.to_string();
                self.settings_state.config = Some(config.clone());
                self.settings_state.original_config = Some(config);
                self.settings_state.loading = false;
                self.settings_state.has_unsaved_changes = false;
            }
            Err(e) => {
                let error_msg = categorize_error(&e.to_string());
                self.settings_state.error = Some(error_msg);
                self.settings_state.loading = false;
            }
        }

        Ok(())
    }

    /// Try to switch tab, checking for unsaved changes
    pub fn try_switch_tab(&mut self, new_tab: super::Tab) {
        // If we're leaving settings with unsaved changes, show confirmation
        if self.current_tab == super::Tab::Settings && self.settings_state.has_unsaved_changes {
            self.settings_state.show_save_confirmation = true;
            self.settings_state.pending_tab = Some(new_tab);
        } else {
            self.current_tab = new_tab;
        }
    }

    /// Confirm tab switch or logout without saving
    pub fn confirm_discard_changes(&mut self) {
        self.settings_state.has_unsaved_changes = false;
        self.settings_state.show_save_confirmation = false;

        if let Some(pending_tab) = self.settings_state.pending_tab.take() {
            // Switch to pending tab
            self.current_tab = pending_tab;
        } else {
            // No pending tab means logout/exit was requested
            // Clear session file
            if let Err(e) = self.config_manager.delete_session(&self.instance_id) {
                eprintln!("Warning: Failed to delete session: {}", e);
            }

            // Reset app state
            self.auth_state.current_user = None;
            self.current_screen = super::Screen::Auth;
            self.posts_state.posts.clear();
            self.profile_state.profile = None;
            self.dms_state.conversations.clear();
            self.dms_state.conversations_loaded = false;
            self.dms_state.messages.clear();
        }
    }

    /// Cancel tab switch
    pub fn cancel_tab_switch(&mut self) {
        self.settings_state.show_save_confirmation = false;
        self.settings_state.pending_tab = None;
    }

    /// Cycle color scheme
    pub fn cycle_color_scheme(&mut self) {
        if let Some(config) = &mut self.settings_state.config {
            config.color_scheme = match config.color_scheme {
                fido_types::ColorScheme::Default => fido_types::ColorScheme::Dark,
                fido_types::ColorScheme::Dark => fido_types::ColorScheme::Light,
                fido_types::ColorScheme::Light => fido_types::ColorScheme::Solarized,
                fido_types::ColorScheme::Solarized => fido_types::ColorScheme::Default,
            };
            self.check_settings_changes();
        }
    }

    /// Cycle color scheme backward
    pub fn cycle_color_scheme_backward(&mut self) {
        if let Some(config) = &mut self.settings_state.config {
            config.color_scheme = match config.color_scheme {
                fido_types::ColorScheme::Default => fido_types::ColorScheme::Solarized,
                fido_types::ColorScheme::Dark => fido_types::ColorScheme::Default,
                fido_types::ColorScheme::Light => fido_types::ColorScheme::Dark,
                fido_types::ColorScheme::Solarized => fido_types::ColorScheme::Light,
            };
            self.check_settings_changes();
        }
    }

    /// Cycle sort order
    pub fn cycle_sort_order(&mut self) {
        if let Some(config) = &mut self.settings_state.config {
            config.sort_order = match config.sort_order {
                fido_types::SortOrder::Newest => fido_types::SortOrder::Popular,
                fido_types::SortOrder::Popular => fido_types::SortOrder::Controversial,
                fido_types::SortOrder::Controversial => fido_types::SortOrder::Newest,
            };
            self.check_settings_changes();
        }
    }

    /// Cycle sort order backward
    pub fn cycle_sort_order_backward(&mut self) {
        if let Some(config) = &mut self.settings_state.config {
            config.sort_order = match config.sort_order {
                fido_types::SortOrder::Newest => fido_types::SortOrder::Controversial,
                fido_types::SortOrder::Popular => fido_types::SortOrder::Newest,
                fido_types::SortOrder::Controversial => fido_types::SortOrder::Popular,
            };
            self.check_settings_changes();
        }
    }

    /// Add digit to max posts input
    pub fn add_digit_to_max_posts(&mut self, c: char) {
        if c.is_ascii_digit() && self.settings_state.max_posts_input.len() < 3 {
            self.settings_state.max_posts_input.push(c);
            self.check_settings_changes();
        }
    }

    /// Remove digit from max posts input
    pub fn remove_digit_from_max_posts(&mut self) {
        if !self.settings_state.max_posts_input.is_empty() {
            self.settings_state.max_posts_input.pop();
            self.check_settings_changes();
        }
    }

    /// Increment max posts display
    pub fn increment_max_posts(&mut self) {
        if let Ok(current) = self.settings_state.max_posts_input.parse::<i32>() {
            let new_value = (current + 1).min(420); // Increment by 1, max 100
            self.settings_state.max_posts_input = new_value.to_string();
            self.check_settings_changes();
        }
    }

    /// Decrement max posts display
    pub fn decrement_max_posts(&mut self) {
        if let Ok(current) = self.settings_state.max_posts_input.parse::<i32>() {
            let new_value = (current - 1).max(1); // Decrement by 1, min 1
            self.settings_state.max_posts_input = new_value.to_string();
            self.check_settings_changes();
        }
    }

    /// Save settings
    pub async fn save_settings(&mut self) -> Result<()> {
        if let Some(config) = &self.settings_state.config {
            self.settings_state.error = None;

            // Validate max posts
            let max_posts = match self.settings_state.max_posts_input.parse::<i32>() {
                Ok(n) if n > 0 => n,
                Ok(n) => {
                    self.settings_state.error = Some(format!(
                        "Validation Error: Max posts must be positive (got {})",
                        n
                    ));
                    return Ok(());
                }
                Err(_) if self.settings_state.max_posts_input.is_empty() => {
                    self.settings_state.error =
                        Some("Validation Error: Max posts cannot be empty".to_string());
                    return Ok(());
                }
                Err(_) => {
                    self.settings_state.error = Some(format!(
                        "Validation Error: '{}' is not a valid number",
                        self.settings_state.max_posts_input
                    ));
                    return Ok(());
                }
            };

            let request = fido_types::UpdateConfigRequest {
                color_scheme: Some(config.color_scheme.as_str().to_string()),
                sort_order: Some(config.sort_order.as_str().to_string()),
                max_posts_display: Some(max_posts),
                emoji_enabled: Some(config.emoji_enabled),
            };

            match self.api_client.update_config(request).await {
                Ok(updated_config) => {
                    self.settings_state.max_posts_input =
                        updated_config.max_posts_display.to_string();
                    self.settings_state.original_max_posts_input =
                        updated_config.max_posts_display.to_string();
                    self.settings_state.config = Some(updated_config.clone());
                    self.settings_state.original_config = Some(updated_config);
                    self.settings_state.has_unsaved_changes = false;
                    self.settings_state.error = Some("✓ Settings saved successfully!".to_string());

                    // Reload posts with new settings (max_posts_display and sort_order)
                    let _ = self.load_posts().await;
                }
                Err(e) => {
                    self.settings_state.error = Some(format!(
                        "Network Error: Failed to save settings: {} (Press 's' to retry)",
                        e
                    ));
                }
            }
        }

        Ok(())
    }

    /// Handle keys for Settings tab
    pub fn handle_settings_keys(&mut self, key: KeyEvent) -> Result<()> {
        // If save confirmation is showing, handle that
        if self.settings_state.show_save_confirmation {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    // Save and switch tabs (will be handled async in main loop)
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.confirm_discard_changes();
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.settings_state.selected_field = match self.settings_state.selected_field {
                    SettingsField::ColorScheme => SettingsField::SortOrder,
                    SettingsField::SortOrder => SettingsField::MaxPosts,
                    SettingsField::MaxPosts => SettingsField::MaxPosts, // Stop at last field
                };
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.settings_state.selected_field = match self.settings_state.selected_field {
                    SettingsField::ColorScheme => SettingsField::ColorScheme, // Stop at first field
                    SettingsField::SortOrder => SettingsField::ColorScheme,
                    SettingsField::MaxPosts => SettingsField::SortOrder,
                };
            }
            KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Left => {
                match self.settings_state.selected_field {
                    SettingsField::ColorScheme => self.cycle_color_scheme_backward(),
                    SettingsField::SortOrder => self.cycle_sort_order_backward(),
                    SettingsField::MaxPosts => self.decrement_max_posts(),
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Right | KeyCode::Enter => {
                match self.settings_state.selected_field {
                    SettingsField::ColorScheme => self.cycle_color_scheme(),
                    SettingsField::SortOrder => self.cycle_sort_order(),
                    SettingsField::MaxPosts => self.increment_max_posts(),
                }
            }
            KeyCode::Backspace if self.settings_state.selected_field == SettingsField::MaxPosts => {
                self.remove_digit_from_max_posts();
            }
            KeyCode::Char(c) if self.settings_state.selected_field == SettingsField::MaxPosts => {
                self.add_digit_to_max_posts(c);
            }
            KeyCode::Char('s') => {
                // Save settings (will be handled async in main loop)
            }
            _ => {}
        }
        Ok(())
    }

    /// Check if settings have changed from original
    fn check_settings_changes(&mut self) {
        if let (Some(current), Some(original)) = (
            &self.settings_state.config,
            &self.settings_state.original_config,
        ) {
            let config_changed = current.color_scheme != original.color_scheme
                || current.sort_order != original.sort_order;
            let max_posts_changed =
                self.settings_state.max_posts_input != self.settings_state.original_max_posts_input;

            self.settings_state.has_unsaved_changes = config_changed || max_posts_changed;
        }
    }
}
