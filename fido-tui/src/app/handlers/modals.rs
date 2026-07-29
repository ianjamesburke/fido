use crate::app::state::{App, FilterTab, Tab};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

/// Handle modal key events, returning Some(()) if a modal consumed the event
pub fn handle_modal_keys(app: &mut App, key: KeyEvent) -> Result<Option<()>> {
    // Community settings modal ('c' claim is async and handled by the event loop)
    if app.show_community_modal {
        if matches!(
            key.code,
            KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I')
        ) {
            app.show_community_modal = false;
        }
        return Ok(Some(()));
    }

    if app.notifications_state.show {
        app.handle_notifications_panel_keys(key)?;
        return Ok(Some(()));
    }

    // Priority 3: Filter modal
    if app.posts_state.show_filter_modal {
        if matches!(key.code, KeyCode::Esc) {
            // If in add hashtag input mode, just close the input, not the whole modal
            if app.posts_state.filter_modal_state.show_add_hashtag_input {
                app.posts_state.filter_modal_state.show_add_hashtag_input = false;
                app.posts_state.filter_modal_state.add_hashtag_input.clear();
                return Ok(Some(()));
            }
            // Otherwise, close the entire filter modal
            app.cancel_filter_modal();
            return Ok(Some(()));
        }
        handle_filter_modal_keys(app, key)?;
        return Ok(Some(()));
    }

    // Priority 4: Unified composer modal
    if app.composer_state.is_open() {
        if matches!(key.code, KeyCode::Esc) {
            app.close_composer();
            return Ok(Some(()));
        }
        // Let Enter key pass through for async processing (submit_composer)
        if matches!(key.code, KeyCode::Enter) {
            return Ok(None); // Let the event loop handle Enter async
        }
        // All other keys are handled by TextArea
        app.handle_composer_input(key);
        return Ok(Some(()));
    }

    if app.dms_state.show_new_conversation_modal {
        if matches!(key.code, KeyCode::Esc) {
            app.close_new_conversation_modal();
            return Ok(Some(()));
        }
        app.handle_new_conversation_modal_keys(key)?;
        return Ok(Some(()));
    }

    // DM error message (not modal, just error text in DMs tab)
    if app.current_tab == Tab::DMs
        && app.dms_state.error.is_some()
        && matches!(key.code, KeyCode::Esc)
    {
        app.clear_dm_error();
        return Ok(Some(()));
    }

    // Friends modal
    if app.friends_state.show_friends_modal {
        if matches!(key.code, KeyCode::Esc) {
            app.close_friends_modal();
            return Ok(Some(()));
        }
        app.handle_friends_modal_keys(key)?;
        return Ok(Some(()));
    }

    // User search modal
    if app.user_search_state.show_modal {
        if matches!(key.code, KeyCode::Esc) {
            app.close_user_search_modal();
            return Ok(Some(()));
        }
        app.handle_user_search_modal_keys(key)?;
        return Ok(Some(()));
    }

    // Hashtags modal
    if app.hashtags_state.show_hashtags_modal {
        if matches!(key.code, KeyCode::Esc) {
            app.close_hashtags_modal();
            return Ok(Some(()));
        }
        app.handle_hashtags_modal_keys(key)?;
        return Ok(Some(()));
    }

    Ok(None)
}

pub fn handle_filter_modal_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    let in_hashtags_tab = app.posts_state.filter_modal_state.selected_tab == FilterTab::Hashtags;
    let in_add_hashtag_input = app.posts_state.filter_modal_state.show_add_hashtag_input;

    if in_add_hashtag_input {
        handle_add_hashtag_input(app, key);
        return Ok(());
    }

    match key.code {
        KeyCode::Esc => {
            app.cancel_filter_modal();
        }
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
            cycle_filter_tabs_forward(app);
        }
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
            cycle_filter_tabs_backward(app);
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            navigate_filter_modal_down(app);
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            navigate_filter_modal_up(app);
        }
        KeyCode::Char(' ') => {
            app.toggle_filter_item();
        }
        KeyCode::Char('x') | KeyCode::Char('X') if in_hashtags_tab => {}
        KeyCode::Enter
            if in_hashtags_tab
                && app.posts_state.filter_modal_state.selected_index
                    == app.posts_state.filter_modal_state.hashtag_list.len() =>
        {
            app.posts_state.filter_modal_state.show_add_hashtag_input = true;
            app.posts_state.filter_modal_state.add_hashtag_input.clear();
        }
        _ => {}
    }
    Ok(())
}

fn handle_add_hashtag_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) => {
            app.posts_state.filter_modal_state.add_hashtag_input.push(c);
        }
        KeyCode::Backspace => {
            app.posts_state.filter_modal_state.add_hashtag_input.pop();
        }
        KeyCode::Esc => {
            app.posts_state.filter_modal_state.show_add_hashtag_input = false;
            app.posts_state.filter_modal_state.add_hashtag_input.clear();
        }
        KeyCode::Enter => {}
        _ => {}
    }
}

fn cycle_filter_tabs_forward(app: &mut App) {
    app.posts_state.filter_modal_state.selected_tab =
        match app.posts_state.filter_modal_state.selected_tab {
            FilterTab::All => FilterTab::Hashtags,
            FilterTab::Hashtags => FilterTab::Users,
            FilterTab::Users => FilterTab::All,
        };
    app.posts_state.filter_modal_state.selected_index = 0;
}

fn cycle_filter_tabs_backward(app: &mut App) {
    app.posts_state.filter_modal_state.selected_tab =
        match app.posts_state.filter_modal_state.selected_tab {
            FilterTab::All => FilterTab::Users,
            FilterTab::Users => FilterTab::Hashtags,
            FilterTab::Hashtags => FilterTab::All,
        };
    app.posts_state.filter_modal_state.selected_index = 0;
}

fn navigate_filter_modal_down(app: &mut App) {
    let max_index = match app.posts_state.filter_modal_state.selected_tab {
        FilterTab::All => 0,
        FilterTab::Hashtags => app.posts_state.filter_modal_state.hashtag_list.len(),
        FilterTab::Users => app
            .posts_state
            .filter_modal_state
            .user_list
            .len()
            .saturating_sub(1),
    };
    if app.posts_state.filter_modal_state.selected_index < max_index {
        app.posts_state.filter_modal_state.selected_index += 1;
    }
}

fn navigate_filter_modal_up(app: &mut App) {
    if app.posts_state.filter_modal_state.selected_index > 0 {
        app.posts_state.filter_modal_state.selected_index -= 1;
    }
}
