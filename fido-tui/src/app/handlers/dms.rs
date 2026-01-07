use crate::app::state::{App, InputMode};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_dms_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.dms_state.show_new_conversation_modal {
        return app.handle_new_conversation_modal_keys(key);
    }

    match app.input_mode {
        InputMode::Navigation => handle_navigation_keys(app, key),
        InputMode::Typing => handle_typing_keys(app, key),
    }
}

fn handle_navigation_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            handle_conversation_navigation_down(app);
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            handle_conversation_navigation_up(app);
        }
        KeyCode::Enter => {
            if app.dms_state.selected_conversation_index == Some(usize::MAX) {
                app.dms_state.show_new_conversation_modal = true;
                app.dms_state.new_conversation_username.clear();
                app.input_mode = InputMode::Typing;
            }
        }
        _ => {
            app.input_mode = InputMode::Typing;
            app.handle_dm_input(key);
            if app.dms_state.messages.is_empty() {
                app.dms_state.needs_message_load = true;
            }
        }
    }
    Ok(())
}

fn handle_typing_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.clear_dm_message();
            app.input_mode = InputMode::Navigation;
        }
        KeyCode::Enter => {}
        _ => {
            app.handle_dm_input(key);
            if app.is_dm_message_empty() && key.code == KeyCode::Backspace {
                app.input_mode = InputMode::Navigation;
            }
        }
    }
    Ok(())
}

fn handle_conversation_navigation_down(app: &mut App) {
    match app.dms_state.selected_conversation_index {
        None => {
            // If there's a pending conversation, clear it and move to "New Conversation" button
            if app.dms_state.pending_conversation_username.is_some() {
                app.dms_state.pending_conversation_username = None;
                app.dms_state.message_input.clear();
            }
            app.dms_state.selected_conversation_index = Some(usize::MAX);
        }
        Some(usize::MAX) => {
            if !app.dms_state.conversations.is_empty() {
                app.dms_state.selected_conversation_index = Some(0);
            }
        }
        Some(index) => {
            if index < app.dms_state.conversations.len().saturating_sub(1) {
                app.dms_state.selected_conversation_index = Some(index + 1);
            }
        }
    }
}

fn handle_conversation_navigation_up(app: &mut App) {
    match app.dms_state.selected_conversation_index {
        None => {
            // If there's a pending conversation, clear it and navigate
            if app.dms_state.pending_conversation_username.is_some() {
                app.dms_state.pending_conversation_username = None;
                app.dms_state.message_input.clear();
            }
            if !app.dms_state.conversations.is_empty() {
                app.dms_state.selected_conversation_index =
                    Some(app.dms_state.conversations.len() - 1);
            } else {
                app.dms_state.selected_conversation_index = Some(usize::MAX);
            }
        }
        Some(0) => {
            app.dms_state.selected_conversation_index = Some(usize::MAX);
        }
        Some(usize::MAX) => {}
        Some(index) => {
            app.dms_state.selected_conversation_index = Some(index - 1);
        }
    }
}
