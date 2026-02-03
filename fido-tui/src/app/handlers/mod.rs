pub mod dms;
pub mod modals;
pub mod posts;
pub mod profile;
pub mod settings;

// Re-export the handler functions that are called from app/mod.rs

use crate::app::state::{App, InputMode, Screen, Tab};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }

    // Priority 0: Ctrl+C always quits (highest priority, works everywhere)
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.running = false;
        return Ok(());
    }

    // Priority 1: Help modal (highest priority)
    if app.show_help {
        return handle_help_keys(app, key);
    }

    // Priority 1.5: User profile view
    if app.user_profile_view.is_some() {
        return app.handle_user_profile_view_keys(key);
    }

    // Priority 2: Save confirmation modal
    if app.settings_state.show_save_confirmation {
        return handle_save_confirmation(app, key);
    }

    // Priority 3: Modal handling
    if let Some(result) = modals::handle_modal_keys(app, key)? {
        return Ok(result);
    }

    // Priority 4: Post detail view modals
    if app.viewing_post_detail {
        if let Some(detail_state) = &app.post_detail_state {
            // Handle delete confirmation modal (HIGHEST priority)
            if detail_state.show_delete_confirmation {
                if matches!(key.code, KeyCode::Esc) {
                    app.cancel_delete_confirmation();
                    return Ok(());
                }
                return app.handle_delete_confirmation_keys(key);
            }

            // Handle full post modal
            if detail_state.show_full_post_modal {
                if matches!(key.code, KeyCode::Esc) {
                    app.close_full_post_modal();
                    return Ok(());
                }
                return posts::handle_full_post_modal_keys(app, key);
            }
        }

        // Handle post detail view ESC (close and return to feed)
        if matches!(key.code, KeyCode::Esc) {
            app.close_post_detail();
            return Ok(());
        }
    }

    // Priority 5: Global keys (quit/exit)
    if handle_global_keys(app, key)? {
        return Ok(());
    }

    // Screen-specific keys
    match app.current_screen {
        Screen::Auth => {
            app.handle_auth_keys(key)?;
        }
        Screen::Main => {
            handle_main_keys(app, key)?;
        }
    }

    Ok(())
}

fn handle_help_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') => {
            app.toggle_help();
        }
        _ => {}
    }
    Ok(())
}

fn handle_save_confirmation(app: &mut App, key: KeyEvent) -> Result<()> {
    if matches!(key.code, KeyCode::Esc) {
        app.cancel_tab_switch();
        return Ok(());
    }
    app.handle_settings_keys(key)
}

fn handle_global_keys(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        // '?' key for help (Shift+/)
        KeyCode::Char('?')
            if !app.composer_state.is_open() && app.input_mode == InputMode::Navigation =>
        {
            app.toggle_help();
            Ok(true)
        }
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc
            if app.input_mode == InputMode::Navigation =>
        {
            // Check for unsaved changes in Settings before exiting
            if app.current_screen == Screen::Main
                && app.current_tab == Tab::Settings
                && app.settings_state.has_unsaved_changes
            {
                app.settings_state.show_save_confirmation = true;
                app.settings_state.pending_tab = None; // None indicates logout/exit
                return Ok(true);
            }
            app.running = false;
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub fn handle_main_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Tab => {
            app.next_tab();
        }
        KeyCode::BackTab => {
            app.previous_tab();
        }
        _ => match app.current_tab {
            Tab::Posts => posts::handle_posts_keys(app, key)?,
            Tab::Profile => profile::handle_profile_keys(app, key)?,
            Tab::DMs => dms::handle_dms_keys(app, key)?,
            Tab::Settings => settings::handle_settings_keys(app, key)?,
        },
    }
    Ok(())
}
