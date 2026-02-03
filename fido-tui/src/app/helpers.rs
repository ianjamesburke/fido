use anyhow::Result;
use crossterm::event::KeyEvent;

use crate::app::{handlers, App, Tab};

impl App {
    /// Toggle help modal
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Clear expired messages (auto-clear after 3 seconds)
    pub fn clear_expired_messages(&mut self) {
        let now = std::time::Instant::now();
        let duration = std::time::Duration::from_secs(3);

        // Clear posts state message if expired
        if let Some((_, timestamp)) = &self.posts_state.message {
            if now.duration_since(*timestamp) > duration {
                self.posts_state.message = None;
            }
        }

        // Clear post detail state message if expired
        if let Some(detail_state) = &mut self.post_detail_state {
            if let Some((_, timestamp)) = &detail_state.message {
                if now.duration_since(*timestamp) > duration {
                    detail_state.message = None;
                }
            }
        }
    }

    /// Check if we need to load data when switching tabs
    pub fn needs_tab_data_load(&self) -> bool {
        matches!(self.current_tab, Tab::Profile | Tab::DMs | Tab::Settings)
    }

    /// Handle keyboard events with priority-based Escape handling
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        handlers::handle_key_event(self, key)
    }
}
