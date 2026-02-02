use crate::app::state::App;
use anyhow::Result;
use crossterm::event::KeyEvent;

/// Handle DM tab keyboard events
/// This delegates to the main App::handle_dms_keys implementation
pub fn handle_dms_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    app.handle_dms_keys(key)
}
