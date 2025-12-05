use crate::app::state::{App, FilterTab, InputMode, Screen, SettingsField, Tab};
use crate::{log_key_event, log_settings};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use fido_types::Post;

pub fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }

    // Priority 1: Help modal (highest priority)
    if app.show_help {
        if matches!(key.code, KeyCode::Esc) {
            app.toggle_help();
            return Ok(());
        }
        // ? to toggle help
        if key.code == KeyCode::Char('?') {
            app.toggle_help();
            return Ok(());
        }
        return Ok(());
    }

    // Priority 1.5: User profile view
    if app.user_profile_view.is_some() {
        return app.handle_user_profile_view_keys(key);
    }

    // Priority 2: Save confirmation modal
    if app.settings_state.show_save_confirmation {
        if matches!(key.code, KeyCode::Esc) {
            app.cancel_tab_switch();
            return Ok(());
        }
        // Handle Y/N for save confirmation
        return app.handle_settings_keys(key);
    }

    // Priority 3: Filter modal
    if app.posts_state.show_filter_modal {
        if matches!(key.code, KeyCode::Esc) {
            // If in add hashtag input mode, just close the input, not the whole modal
            if app.posts_state.filter_modal_state.show_add_hashtag_input {
                app.posts_state.filter_modal_state.show_add_hashtag_input = false;
                app.posts_state.filter_modal_state.add_hashtag_input.clear();
                return Ok(());
            }
            // Otherwise, close the entire filter modal
            app.cancel_filter_modal();
            return Ok(());
        }
        return app.handle_filter_modal_keys(key);
    }

    // Priority 4: Unified composer modal
    if app.composer_state.is_open() {
        if matches!(key.code, KeyCode::Esc) {
            app.close_composer();
            return Ok(());
        }
        // All other keys are handled by TextArea
        app.handle_composer_input(key);
        return Ok(());
    }

    if app.dms_state.show_new_conversation_modal {
        if matches!(key.code, KeyCode::Esc) {
            app.close_new_conversation_modal();
            return Ok(());
        }
        return app.handle_new_conversation_modal_keys(key);
    }

    // Priority: DM error modal
    if app.dms_state.show_dm_error_modal {
        if matches!(key.code, KeyCode::Esc) {
            app.close_dm_error_modal();
            return Ok(());
        }
        return app.handle_dm_error_modal_keys(key);
    }

    // Priority: DM error message (not modal, just error text in DMs tab)
    // Note: Unlike modal errors, this allows normal navigation but clears on Esc
    if app.current_tab == Tab::DMs && app.dms_state.error.is_some()
        && matches!(key.code, KeyCode::Esc) {
            app.clear_dm_error();
            return Ok(());
        }
        // Allow other keys to pass through for normal DM interaction

    // Priority: Friends modal
    if app.friends_state.show_friends_modal {
        if matches!(key.code, KeyCode::Esc) {
            app.close_friends_modal();
            return Ok(());
        }
        return app.handle_friends_modal_keys(key);
    }

    // Priority: Hashtags modal
    if app.hashtags_state.show_hashtags_modal {
        if matches!(key.code, KeyCode::Esc) {
            app.close_hashtags_modal();
            return Ok(());
        }
        return app.handle_hashtags_modal_keys(key);
    }

    // Priority 4: Post detail view modals
    // IMPORTANT: Don't handle modal keys if composer is open (composer has priority)
    if app.viewing_post_detail && !app.composer_state.is_open() {
        if let Some(detail_state) = &app.post_detail_state {
            // Handle delete confirmation modal (HIGHEST priority - must be checked first)
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
                return app.handle_full_post_modal_keys(key);
            }
        }

        // Handle post detail view ESC (close and return to feed)
        if matches!(key.code, KeyCode::Esc) {
            app.close_post_detail();
            return Ok(());
        }
    }

    // Priority 5: Global keys (quit/exit)
    match key.code {
        // '?' key for help (Shift+/)
        KeyCode::Char('?')
            if !app.composer_state.is_open() && app.input_mode == InputMode::Navigation =>
        {
            app.toggle_help();
            return Ok(());
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
                return Ok(());
            }
            app.running = false;
            return Ok(());
        }
        _ => {}
    }

    // Screen-specific keys (let tabs handle keys first)
    let _handled = match app.current_screen {
        Screen::Auth => {
            app.handle_auth_keys(key)?;
            true
        }
        Screen::Main => {
            app.handle_main_keys(key)?;
            // Check if the key was actually handled by checking if mode changed
            // If we're now in Typing mode, the key was consumed for typing
            app.input_mode == InputMode::Typing
        }
    };

    Ok(())
}

pub fn handle_main_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    // Debug logging for h/H/l/L keys
    if matches!(key.code, KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('l') | KeyCode::Char('L')) {
        log_key_event!(app.log_config, "handle_main_keys received key: {:?}, current_tab: {:?}", key.code, app.current_tab);
    }
    
    match key.code {
        KeyCode::Tab => {
            app.next_tab();
        }
        KeyCode::BackTab => {
            app.previous_tab();
        }
        // Shift+L (logout) is handled in main.rs as an async operation
        _ => match app.current_tab {
            Tab::Posts => app.handle_posts_keys(key)?,
            Tab::Profile => app.handle_profile_keys(key)?,
            Tab::DMs => app.handle_dms_keys(key)?,
            Tab::Settings => {
                log_settings!(app.log_config, "Calling handle_settings_keys");
                app.handle_settings_keys(key)?;
            },
        },
    }
    Ok(())
}

pub fn handle_posts_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.posts_state.show_new_post_modal {
        return app.handle_new_post_modal_keys(key);
    }

    if app.viewing_post_detail {
        return app.handle_post_detail_keys(key);
    }

    match key.code {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            app.next_post();
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            app.previous_post();
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {}
        KeyCode::Char('d') | KeyCode::Char('D') => {}
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.open_composer_new_post();
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            app.open_filter_modal();
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {}
        KeyCode::Enter => {}
        _ => {}
    }
    Ok(())
}

pub fn handle_profile_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.profile_state.show_edit_bio_modal {
        return app.handle_edit_bio_modal_keys(key);
    }

    match key.code {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => app.next_user_post(),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => app.previous_user_post(),
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

pub fn handle_dms_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.dms_state.show_new_conversation_modal {
        return app.handle_new_conversation_modal_keys(key);
    }

    match app.input_mode {
        InputMode::Navigation => match key.code {
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => match app.dms_state.selected_conversation_index {
                None => {
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
            },
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => match app.dms_state.selected_conversation_index {
                None => {
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
            },
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
        },
        InputMode::Typing => match key.code {
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
        },
    }
    Ok(())
}

pub fn handle_settings_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    // Debug logging - log ALL keys
    log_settings!(app.log_config, "handle_settings_keys START: key={:?}", key.code);
    
    if app.settings_state.show_save_confirmation {
        log_settings!(app.log_config, "In save confirmation mode");

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {}
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.confirm_discard_changes();
            }
            _ => {}
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            app.settings_state.selected_field = match app.settings_state.selected_field {
                SettingsField::ColorScheme => SettingsField::SortOrder,
                SettingsField::SortOrder => SettingsField::MaxPosts,
                SettingsField::MaxPosts => SettingsField::MaxPosts,
            };
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            app.settings_state.selected_field = match app.settings_state.selected_field {
                SettingsField::ColorScheme => SettingsField::ColorScheme,
                SettingsField::SortOrder => SettingsField::ColorScheme,
                SettingsField::MaxPosts => SettingsField::SortOrder,
            };
        }
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Left => {
            log_settings!(app.log_config, "Matched h/H/Left pattern, field: {:?}", app.settings_state.selected_field);
            match app.settings_state.selected_field {
                SettingsField::ColorScheme => app.cycle_color_scheme_backward(),
                SettingsField::SortOrder => app.cycle_sort_order_backward(),
                SettingsField::MaxPosts => app.decrement_max_posts(),
            }
        },
        KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Right | KeyCode::Enter => match app.settings_state.selected_field {
            SettingsField::ColorScheme => app.cycle_color_scheme(),
            SettingsField::SortOrder => app.cycle_sort_order(),
            SettingsField::MaxPosts => app.increment_max_posts(),
        },
        KeyCode::Backspace if app.settings_state.selected_field == SettingsField::MaxPosts => {
            app.remove_digit_from_max_posts();
        }
        KeyCode::Char(c) if app.settings_state.selected_field == SettingsField::MaxPosts => {
            app.add_digit_to_max_posts(c);
        }
        KeyCode::Char('s') => {}
        _ => {}
    }
    Ok(())
}

pub fn handle_filter_modal_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    let in_hashtags_tab = app.posts_state.filter_modal_state.selected_tab == FilterTab::Hashtags;
    let in_add_hashtag_input = app.posts_state.filter_modal_state.show_add_hashtag_input;

    if in_add_hashtag_input {
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
        return Ok(());
    }

    match key.code {
        KeyCode::Esc => {
            app.cancel_filter_modal();
        }
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
            app.posts_state.filter_modal_state.selected_tab =
                match app.posts_state.filter_modal_state.selected_tab {
                    FilterTab::All => FilterTab::Hashtags,
                    FilterTab::Hashtags => FilterTab::Users,
                    FilterTab::Users => FilterTab::All,
                };
            app.posts_state.filter_modal_state.selected_index = 0;
        }
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
            app.posts_state.filter_modal_state.selected_tab =
                match app.posts_state.filter_modal_state.selected_tab {
                    FilterTab::All => FilterTab::Users,
                    FilterTab::Users => FilterTab::Hashtags,
                    FilterTab::Hashtags => FilterTab::All,
                };
            app.posts_state.filter_modal_state.selected_index = 0;
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
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
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if app.posts_state.filter_modal_state.selected_index > 0 {
                app.posts_state.filter_modal_state.selected_index -= 1;
            }
        }
        KeyCode::Char(' ') => {
            app.toggle_filter_item();
        }
        KeyCode::Char('x') | KeyCode::Char('X') if in_hashtags_tab => {}
        KeyCode::Enter => {
            if in_hashtags_tab
                && app.posts_state.filter_modal_state.selected_index
                    == app.posts_state.filter_modal_state.hashtag_list.len()
            {
                app.posts_state.filter_modal_state.show_add_hashtag_input = true;
                app.posts_state.filter_modal_state.add_hashtag_input.clear();
            } 
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_post_detail_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    if let Some(detail_state) = &app.post_detail_state {
        if detail_state.show_full_post_modal {
            return handle_full_post_modal_keys(app, key);
        }
        if detail_state.show_reply_composer {
            return handle_reply_composer_keys(app, key);
        }
        if detail_state.show_delete_confirmation {
            return app.handle_delete_confirmation_keys(key);
        }
    }

    match key.code {
        KeyCode::Enter => {
            app.open_full_post_modal();
        }
        KeyCode::Esc => {
            app.close_post_detail();
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            if let Some(detail_state) = &mut app.post_detail_state {
                let direct_reply_count = detail_state
                    .replies
                    .iter()
                    .filter(|reply| {
                        if let Some(parent_id) = reply.parent_post_id {
                            !detail_state.replies.iter().any(|r| r.id == parent_id)
                        } else {
                            false
                        }
                    })
                    .count();

                if direct_reply_count > 0 {
                    let next_index = match detail_state.reply_list_state.selected() {
                        None => 0,
                        Some(i) if i < direct_reply_count - 1 => i + 1,
                        Some(i) => i,
                    };
                    detail_state.reply_list_state.select(Some(next_index));
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if let Some(detail_state) = &mut app.post_detail_state {
                let direct_reply_count = detail_state
                    .replies
                    .iter()
                    .filter(|reply| {
                        if let Some(parent_id) = reply.parent_post_id {
                            !detail_state.replies.iter().any(|r| r.id == parent_id)
                        } else {
                            false
                        }
                    })
                    .count();

                if direct_reply_count > 0 {
                    match detail_state.reply_list_state.selected() {
                        None => {}
                        Some(0) => {
                            detail_state.reply_list_state.select(None);
                        }
                        Some(i) => {
                            detail_state.reply_list_state.select(Some(i - 1));
                        }
                    }
                }
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(detail_state) = &app.post_detail_state {
                if let Some(post) = &detail_state.post {
                    app.open_composer_reply(
                        post.id,
                        post.author_username.clone(),
                        post.content.clone(),
                    );
                }
            }
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if let Some(detail_state) = &app.post_detail_state {
                if let Some(selected_reply_index) = detail_state.reply_list_state.selected() {
                    let direct_replies: Vec<&Post> = detail_state
                        .replies
                        .iter()
                        .filter(|reply| {
                            if let Some(parent_id) = reply.parent_post_id {
                                !detail_state.replies.iter().any(|r| r.id == parent_id)
                            } else {
                                false
                            }
                        })
                        .collect();

                    if let Some(reply) = direct_replies.get(selected_reply_index) {
                        app.open_composer_reply(
                            reply.id,
                            reply.author_username.clone(),
                            reply.content.clone(),
                        );
                    }
                }
            }
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            app.show_delete_confirmation();
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {}
        KeyCode::Char('d') | KeyCode::Char('D') => {}
        KeyCode::Char('p') | KeyCode::Char('P') => {}
        _ => {}
    }
    Ok(())
}

pub fn handle_full_post_modal_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.close_full_post_modal();
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            app.modal_next_reply();
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            app.modal_previous_reply();
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            app.modal_toggle_expansion();
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(detail_state) = &app.post_detail_state {
                if let Some(modal_post_id) = detail_state.full_post_modal_id {
                    let post_to_reply = if let Some(post) = &detail_state.post {
                        if post.id == modal_post_id {
                            Some(post.clone())
                        } else {
                            detail_state
                                .replies
                                .iter()
                                .find(|r| r.id == modal_post_id)
                                .cloned()
                        }
                    } else {
                        None
                    };

                    if let Some(post) = post_to_reply {
                        // Don't close the modal - keep it open so we return to it after submitting
                        app.open_composer_reply(
                            post.id,
                            post.author_username.clone(),
                            post.content.clone(),
                        );
                    }
                }
            }
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {}
        KeyCode::Char('u') | KeyCode::Char('U') => {}
        KeyCode::Char('d') | KeyCode::Char('D') => {}
        KeyCode::Char('x') | KeyCode::Char('X') => {
            app.show_delete_confirmation();
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_reply_composer_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char(c) => {
            app.add_char_to_reply(c);
        }
        KeyCode::Backspace => {
            app.remove_char_from_reply();
        }
        KeyCode::Enter
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) => {}
        KeyCode::Esc => {
            app.close_reply_composer();
        }
        _ => {}
    }
    Ok(())
}
