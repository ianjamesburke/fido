use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use super::App;

impl App {
    /// Handle keyboard events for user profile view
    pub fn handle_user_profile_view_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close_user_profile_view();
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Toggle follow/unfollow (will be handled async in main loop)
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                // Open DM if mutual friends (will be handled async in main loop)
            }
            _ => {}
        }
        Ok(())
    }
}
