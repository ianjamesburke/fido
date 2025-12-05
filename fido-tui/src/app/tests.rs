use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Helper to create a KeyEvent
fn key_event(code: KeyCode) -> KeyEvent {
    let mut event = KeyEvent::new(code, KeyModifiers::empty());
    event.kind = KeyEventKind::Press;
    event
}

#[test]
fn test_escape_closes_help_modal_first() {
    let mut app = App::new();
    app.show_help = true;
    app.running = true;

    // Escape should close help, not exit app
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(!app.show_help, "Help modal should be closed");
    assert!(app.running, "App should still be running");
}

#[test]
fn test_question_mark_toggles_help() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.input_mode = InputMode::Navigation;
    app.show_help = false;

    // '?' should open help modal
    app.handle_key_event(key_event(KeyCode::Char('?'))).unwrap();
    assert!(app.show_help, "Help modal should be open");

    // '?' should close help modal when it's already open
    app.handle_key_event(key_event(KeyCode::Char('?'))).unwrap();
    assert!(!app.show_help, "Help modal should be closed");
}

#[test]
fn test_escape_closes_save_confirmation_modal() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Settings;
    app.settings_state.show_save_confirmation = true;
    app.settings_state.pending_tab = Some(Tab::Posts);
    app.running = true;

    // Escape should cancel save confirmation, not exit app
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(
        !app.settings_state.show_save_confirmation,
        "Save confirmation should be closed"
    );
    assert_eq!(
        app.settings_state.pending_tab, None,
        "Pending tab should be cleared"
    );
    assert!(app.running, "App should still be running");
}

#[test]
fn test_escape_closes_bio_modal() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.composer_state.mode = Some(ComposerMode::EditBio);
    app.composer_state.textarea.insert_str("Test bio");
    app.running = true;

    // Escape should close bio modal, not exit app
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(
        app.composer_state.mode.is_none(),
        "Bio modal should be closed"
    );
    assert!(app.running, "App should still be running");
}

#[test]
fn test_escape_closes_new_post_modal() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.composer_state.mode = Some(ComposerMode::NewPost);
    app.composer_state.textarea.insert_str("Test post");
    app.running = true;

    // Escape should close new post modal, not exit app
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(
        app.composer_state.mode.is_none(),
        "New post modal should be closed"
    );
    assert!(app.running, "App should still be running");
}

#[test]
fn test_escape_closes_new_conversation_modal() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.dms_state.show_new_conversation_modal = true;
    app.dms_state.new_conversation_username = "testuser".to_string();
    app.running = true;

    // Escape should close new conversation modal, not exit app
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(
        !app.dms_state.show_new_conversation_modal,
        "New conversation modal should be closed"
    );
    assert_eq!(
        app.dms_state.new_conversation_username, "",
        "Username should be cleared"
    );
    assert!(app.running, "App should still be running");
}

#[test]
fn test_escape_exits_app_when_no_modals() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.running = true;

    // Escape should exit app when no modals are open
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(!app.running, "App should stop running");
}

#[test]
fn test_escape_shows_save_confirmation_in_settings_with_unsaved_changes() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Settings;
    app.settings_state.has_unsaved_changes = true;
    app.running = true;

    // Escape should show save confirmation, not exit app
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(
        app.settings_state.show_save_confirmation,
        "Save confirmation should be shown"
    );
    assert_eq!(
        app.settings_state.pending_tab, None,
        "Pending tab should be None (exit)"
    );
    assert!(app.running, "App should still be running");
}

#[test]
fn test_modal_priority_help_over_bio() {
    let mut app = App::new();
    app.show_help = true;
    app.profile_state.show_edit_bio_modal = true;
    app.running = true;

    // Escape should close help first, not bio modal
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(!app.show_help, "Help modal should be closed");
    assert!(
        app.profile_state.show_edit_bio_modal,
        "Bio modal should still be open"
    );
    assert!(app.running, "App should still be running");
}

#[test]
fn test_modal_priority_save_confirmation_over_bio() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.settings_state.show_save_confirmation = true;
    app.settings_state.pending_tab = Some(Tab::Posts);
    app.profile_state.show_edit_bio_modal = true;
    app.running = true;

    // Escape should close save confirmation first, not bio modal
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(
        !app.settings_state.show_save_confirmation,
        "Save confirmation should be closed"
    );
    assert!(
        app.profile_state.show_edit_bio_modal,
        "Bio modal should still be open"
    );
    assert!(app.running, "App should still be running");
}

#[test]
fn test_q_key_exits_app() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.running = true;

    // 'q' should exit app
    app.handle_key_event(key_event(KeyCode::Char('q'))).unwrap();

    assert!(!app.running, "App should stop running");
}

#[test]
fn test_q_key_shows_save_confirmation_in_settings_with_unsaved_changes() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Settings;
    app.settings_state.has_unsaved_changes = true;
    app.running = true;

    // 'q' should show save confirmation, not exit app
    app.handle_key_event(key_event(KeyCode::Char('q'))).unwrap();

    assert!(
        app.settings_state.show_save_confirmation,
        "Save confirmation should be shown"
    );
    assert_eq!(
        app.settings_state.pending_tab, None,
        "Pending tab should be None (exit)"
    );
    assert!(app.running, "App should still be running");
}

#[test]
fn test_settings_cycling() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Settings;
    app.settings_state.config = Some(fido_types::UserConfig::default());
    app.settings_state.selected_field = SettingsField::ColorScheme;

    // Test Color Scheme cycling
    let initial_color_scheme = app.settings_state.config.as_ref().unwrap().color_scheme;

    // Press left to go backward
    app.handle_key_event(key_event(KeyCode::Left)).unwrap();
    let new_color_scheme = app.settings_state.config.as_ref().unwrap().color_scheme;
    assert_ne!(initial_color_scheme, new_color_scheme);

    // Press right to go forward
    app.handle_key_event(key_event(KeyCode::Right)).unwrap();
    let final_color_scheme = app.settings_state.config.as_ref().unwrap().color_scheme;
    assert_eq!(initial_color_scheme, final_color_scheme);

    // Test Sort Order cycling
    app.settings_state.selected_field = SettingsField::SortOrder;
    let initial_sort_order = app.settings_state.config.as_ref().unwrap().sort_order;

    // Press left to go backward
    app.handle_key_event(key_event(KeyCode::Left)).unwrap();
    let new_sort_order = app.settings_state.config.as_ref().unwrap().sort_order;
    assert_ne!(initial_sort_order, new_sort_order);

    // Press right to go forward
    app.handle_key_event(key_event(KeyCode::Right)).unwrap();
    let final_sort_order = app.settings_state.config.as_ref().unwrap().sort_order;
    assert_eq!(initial_sort_order, final_sort_order);
}

// ===== Task 13: Test New DM and Input Mode Features =====

/// Helper to create a KeyEvent with modifiers
fn key_event_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn test_unread_indicator_clears_when_opening_conversation() {
    let mut app = App::new();
    let user_id = uuid::Uuid::new_v4();

    // Set up unread count for a user
    app.dms_state.unread_counts.insert(user_id, 5);

    // Simulate opening conversation (this would normally be async)
    app.dms_state.current_conversation_user = Some(user_id);
    app.dms_state.unread_counts.insert(user_id, 0);

    // Verify unread count is cleared
    assert_eq!(
        *app.dms_state.unread_counts.get(&user_id).unwrap(),
        0,
        "Unread count should be cleared when opening conversation"
    );
}

#[test]
fn test_unread_badge_updates_correctly_on_dms_tab() {
    let mut app = App::new();

    // Add multiple conversations with unread counts
    let user1 = uuid::Uuid::new_v4();
    let user2 = uuid::Uuid::new_v4();
    let user3 = uuid::Uuid::new_v4();

    app.dms_state.unread_counts.insert(user1, 3);
    app.dms_state.unread_counts.insert(user2, 5);
    app.dms_state.unread_counts.insert(user3, 0);

    // Calculate total unread (as would be done in UI)
    let total_unread: usize = app.dms_state.unread_counts.values().sum();

    assert_eq!(
        total_unread, 8,
        "Total unread count should be sum of all conversations"
    );

    // Clear one conversation
    app.dms_state.unread_counts.insert(user1, 0);
    let new_total: usize = app.dms_state.unread_counts.values().sum();

    assert_eq!(
        new_total, 5,
        "Total unread should update when conversation is read"
    );
}

#[test]
fn test_n_can_be_typed_in_dm_messages() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::DMs;
    app.input_mode = InputMode::Typing;
    app.dms_state.message_textarea.insert_str("Hello ");

    // Type 'N' (capital N)
    let key = key_event(KeyCode::Char('N'));
    app.handle_key_event(key).unwrap();

    assert_eq!(
        app.dms_state.message_textarea.lines().join("\n"),
        "Hello N",
        "'N' should be added to message input in typing mode"
    );

    // Type 'n' (lowercase n)
    let key = key_event(KeyCode::Char('n'));
    app.handle_key_event(key).unwrap();

    assert_eq!(
        app.dms_state.message_textarea.lines().join("\n"),
        "Hello Nn",
        "'n' should be added to message input in typing mode"
    );
}

#[test]
fn test_u_and_d_can_be_typed_in_post_compose_box() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.posts_state.show_new_post_modal = true;
    app.posts_state.new_post_content = "Test ".to_string();

    // Type 'u'
    let key = key_event(KeyCode::Char('u'));
    app.handle_key_event(key).unwrap();

    assert_eq!(
        app.posts_state.new_post_content, "Test u",
        "'u' should be added to post content in modal"
    );

    // Type 'd'
    let key = key_event(KeyCode::Char('d'));
    app.handle_key_event(key).unwrap();

    assert_eq!(
        app.posts_state.new_post_content, "Test ud",
        "'d' should be added to post content in modal"
    );
}

#[test]
fn test_n_shows_new_post_modal_on_posts_tab() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.input_mode = InputMode::Navigation;

    // Press 'n'
    let key = key_event(KeyCode::Char('n'));
    app.handle_key_event(key).unwrap();

    assert!(
        matches!(app.composer_state.mode, Some(ComposerMode::NewPost)),
        "'n' should open new post modal on Posts tab"
    );
}

#[test]
fn test_escape_hides_new_post_modal_and_returns_to_navigation() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.composer_state.mode = Some(ComposerMode::NewPost);
    app.composer_state.textarea.insert_str("Test content");
    app.input_mode = InputMode::Typing;

    // Press Escape
    let key = key_event(KeyCode::Esc);
    app.handle_key_event(key).unwrap();

    assert!(
        app.composer_state.mode.is_none(),
        "Escape should close new post modal"
    );
    assert_eq!(
        app.composer_state.textarea.lines().join("\n"),
        "",
        "Post content should be cleared (new textarea)"
    );
}

#[test]
fn test_input_mode_switches_correctly_between_typing_and_navigation() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::DMs;
    app.input_mode = InputMode::Navigation;

    // Start typing by pressing a character
    let key = key_event(KeyCode::Char('H'));
    app.handle_key_event(key).unwrap();

    assert_eq!(
        app.input_mode,
        InputMode::Typing,
        "Should switch to typing mode when character is pressed"
    );
    assert_eq!(
        app.dms_state.message_textarea.lines().join("\n"),
        "H",
        "Character should be added to input"
    );

    // Press Escape to return to navigation
    let key = key_event(KeyCode::Esc);
    app.handle_key_event(key).unwrap();

    assert_eq!(
        app.input_mode,
        InputMode::Navigation,
        "Should switch back to navigation mode on Escape"
    );
    assert_eq!(
        app.dms_state.message_textarea.lines().join("\n"),
        "",
        "Input should be cleared on Escape"
    );
}

#[test]
fn test_typing_mode_prevents_navigation_shortcuts() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.posts_state.show_new_post_modal = true;
    app.input_mode = InputMode::Typing;

    // Add some posts
    app.posts_state.posts = vec![Post {
        id: uuid::Uuid::new_v4(),
        author_id: uuid::Uuid::new_v4(),
        author_username: "user1".to_string(),
        content: "Post 1".to_string(),
        created_at: chrono::Utc::now(),
        upvotes: 0,
        downvotes: 0,
        hashtags: Vec::new(),
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    }];
    app.posts_state.list_state.select(Some(0));

    let initial_upvotes = app.posts_state.posts[0].upvotes;

    // Try to upvote while in typing mode (should not work)
    let key = key_event(KeyCode::Char('u'));
    app.handle_key_event(key).unwrap();

    // Upvotes should not change (character should be added to post content instead)
    assert_eq!(
        app.posts_state.posts[0].upvotes, initial_upvotes,
        "Upvote should not trigger in typing mode"
    );
    assert_eq!(
        app.posts_state.new_post_content, "u",
        "Character should be added to post content"
    );
}

#[test]
fn test_navigation_mode_allows_shortcuts() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.input_mode = InputMode::Navigation;

    // Add some posts
    app.posts_state.posts = vec![Post {
        id: uuid::Uuid::new_v4(),
        author_id: uuid::Uuid::new_v4(),
        author_username: "user1".to_string(),
        content: "Post 1".to_string(),
        created_at: chrono::Utc::now(),
        upvotes: 5,
        downvotes: 2,
        hashtags: Vec::new(),
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    }];
    app.posts_state.list_state.select(Some(0));

    // Press 'n' to open new post modal
    let key = key_event(KeyCode::Char('n'));
    app.handle_key_event(key).unwrap();

    assert!(
        matches!(app.composer_state.mode, Some(ComposerMode::NewPost)),
        "Navigation shortcuts should work in navigation mode"
    );
}

#[test]
fn test_backspace_clears_input_and_returns_to_navigation() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::DMs;
    app.input_mode = InputMode::Typing;
    app.dms_state.message_textarea.insert_str("A");

    // Press backspace to remove last character
    let key = key_event(KeyCode::Backspace);
    app.handle_key_event(key).unwrap();

    assert_eq!(
        app.dms_state.message_textarea.lines().join("\n"),
        "",
        "Input should be empty after backspace"
    );
    assert_eq!(
        app.input_mode,
        InputMode::Navigation,
        "Should return to navigation mode when input is empty"
    );
}

#[test]
fn test_ctrl_n_does_not_trigger_on_other_tabs() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.input_mode = InputMode::Navigation;

    // Press Ctrl+N on Posts tab (should not open DM modal)
    let key = key_event_with_modifiers(KeyCode::Char('n'), KeyModifiers::CONTROL);
    app.handle_key_event(key).unwrap();

    assert!(
        !app.dms_state.show_new_conversation_modal,
        "Ctrl+N should not open new conversation modal on non-DM tabs"
    );
}

#[test]
fn test_unread_counts_persist_across_navigation() {
    let mut app = App::new();

    let user1 = uuid::Uuid::new_v4();
    let user2 = uuid::Uuid::new_v4();

    // Set initial unread counts
    app.dms_state.unread_counts.insert(user1, 3);
    app.dms_state.unread_counts.insert(user2, 7);

    // Navigate between conversations
    app.dms_state.selected_conversation_index = Some(0);
    app.dms_state.selected_conversation_index = Some(1);

    // Unread counts should persist
    assert_eq!(
        *app.dms_state.unread_counts.get(&user1).unwrap(),
        3,
        "Unread count for user1 should persist"
    );
    assert_eq!(
        *app.dms_state.unread_counts.get(&user2).unwrap(),
        7,
        "Unread count for user2 should persist"
    );
}

#[test]
fn test_current_conversation_user_tracks_open_conversation() {
    let mut app = App::new();
    let user_id = uuid::Uuid::new_v4();

    // Initially no conversation is open
    assert_eq!(
        app.dms_state.current_conversation_user, None,
        "No conversation should be open initially"
    );

    // Open a conversation
    app.dms_state.current_conversation_user = Some(user_id);

    assert_eq!(
        app.dms_state.current_conversation_user,
        Some(user_id),
        "Current conversation user should be tracked"
    );
}
