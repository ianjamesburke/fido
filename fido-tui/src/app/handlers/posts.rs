use crate::app::state::App;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use fido_types::Post;

pub fn handle_posts_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.posts_state.show_new_post_modal {
        return app.handle_new_post_modal_keys(key);
    }

    if app.viewing_post_detail {
        return handle_post_detail_keys(app, key);
    }

    // Home mode: the Posts tab shows the joined-communities list.
    if app.is_home_list_active() {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                app.next_home_community();
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                app.previous_home_community();
            }
            _ => {}
        }
        return Ok(());
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
        KeyCode::Char('s') | KeyCode::Char('S') => {
            app.open_user_search_modal();
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            if app.community.is_some() {
                app.show_community_modal = true;
            }
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {}
        KeyCode::Enter => {}
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

fn handle_reply_composer_keys(app: &mut App, key: KeyEvent) -> Result<()> {
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
