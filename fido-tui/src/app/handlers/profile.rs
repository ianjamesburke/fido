use crate::app::state::App;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_profile_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.profile_state.show_edit_bio_modal {
        return app.handle_edit_bio_modal_keys(key);
    }

    match key.code {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            app.next_user_post();
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            app.previous_user_post();
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if let Some(profile) = &app.profile_state.profile {
                let current_bio = profile.bio.clone().unwrap_or_default();
                app.open_composer_edit_bio(current_bio);
            }
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            app.friends_state.show_friends_modal = true;
            app.friends_state.selected_index = 0;
            app.friends_state.error = None;
        }
        _ => {}
    }
    Ok(())
}
