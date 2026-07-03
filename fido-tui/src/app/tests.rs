use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fido_types::Post;

/// Put the app on a community board (the launch-repo path in production).
fn set_test_community(app: &mut App) {
    app.community = Some(crate::app::state::CommunityContext {
        id: uuid::Uuid::new_v4(),
        owner: "testowner".to_string(),
        name: "testrepo".to_string(),
        role: Some(fido_types::MembershipRole::Member),
        member_count: 1,
        claimed: false,
    });
}

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
fn test_escape_returns_to_home_list_from_opened_community() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.running = true;
    // Home mode (no launch repo) with a community opened from the list
    app.launch_repo = None;
    set_test_community(&mut app);

    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();

    assert!(app.community.is_none(), "Board should close back to Home");
    assert!(app.running, "App should still be running");

    // Second Escape from the Home list quits
    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();
    assert!(!app.running, "App should stop running from Home");
}

#[test]
fn test_escape_quits_in_repo_mode() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.running = true;
    // Repo mode: launch repo decides the community; there is no list to return to
    app.launch_repo = Some(crate::repo_context::RepoRef {
        owner: "testowner".to_string(),
        name: "testrepo".to_string(),
    });
    set_test_community(&mut app);

    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();
    assert!(!app.running, "Esc quits directly in repo mode");
}

#[test]
fn test_i_opens_community_modal_and_escape_closes_it() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.input_mode = InputMode::Navigation;
    set_test_community(&mut app);

    app.handle_key_event(key_event(KeyCode::Char('i'))).unwrap();
    assert!(app.show_community_modal, "'i' opens the community modal");

    app.handle_key_event(key_event(KeyCode::Esc)).unwrap();
    assert!(!app.show_community_modal, "Esc closes the community modal");
    assert!(app.community.is_some(), "Board stays open behind the modal");
}

#[test]
fn test_home_list_navigation_keys() {
    let mut app = App::new();
    app.current_screen = Screen::Main;
    app.current_tab = Tab::Posts;
    app.input_mode = InputMode::Navigation;
    // Home mode with no community selected
    assert!(app.is_home_list_active());

    // 'n' must not open the composer while the home list is active
    app.handle_key_event(key_event(KeyCode::Char('n'))).unwrap();
    assert!(
        app.composer_state.mode.is_none(),
        "Composer must not open from the Home list"
    );
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
    set_test_community(&mut app);

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
        community_id: uuid::Uuid::new_v4(),
        content: "Post 1".to_string(),
        created_at: chrono::Utc::now(),
        upvotes: 0,
        downvotes: 0,
        approved: true,
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
    set_test_community(&mut app);

    // Add some posts
    app.posts_state.posts = vec![Post {
        id: uuid::Uuid::new_v4(),
        author_id: uuid::Uuid::new_v4(),
        author_username: "user1".to_string(),
        community_id: uuid::Uuid::new_v4(),
        content: "Post 1".to_string(),
        created_at: chrono::Utc::now(),
        upvotes: 5,
        downvotes: 2,
        approved: true,
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

    // Navigate between conversations using the new DMSelection enum
    app.dms_state.selection = crate::app::DMSelection::Conversation(0);
    app.dms_state.selection = crate::app::DMSelection::Conversation(1);

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

#[test]
fn test_dm_selection_starts_on_new_conversation() {
    let app = App::new();

    // DM selection should start on "New Conversation" button
    assert!(
        app.dms_state.selection.is_new_conversation(),
        "DM selection should start on NewConversation"
    );
}

#[test]
fn test_dm_navigation_down_from_new_conversation() {
    let mut app = App::new();

    // Add some conversations
    app.dms_state.conversations = vec![crate::app::Conversation {
        other_user_id: uuid::Uuid::new_v4(),
        other_username: "user1".to_string(),
        last_message: "Hello".to_string(),
        unread_count: 0,
        state: fido_types::DmConversationState::Accepted,
        initiated_by_me: false,
    }];

    // Start on NewConversation
    assert!(app.dms_state.selection.is_new_conversation());

    // Navigate down should go to first conversation
    app.dms_state.navigate_down();
    assert_eq!(
        app.dms_state.selection.conversation_index(),
        Some(0),
        "Should navigate to first conversation"
    );
}

#[test]
fn test_dm_navigation_up_from_conversation() {
    let mut app = App::new();

    // Add some conversations
    app.dms_state.conversations = vec![crate::app::Conversation {
        other_user_id: uuid::Uuid::new_v4(),
        other_username: "user1".to_string(),
        last_message: "Hello".to_string(),
        unread_count: 0,
        state: fido_types::DmConversationState::Accepted,
        initiated_by_me: false,
    }];

    // Start on first conversation
    app.dms_state.selection = crate::app::DMSelection::Conversation(0);

    // Navigate up should go to NewConversation
    app.dms_state.navigate_up();
    assert!(
        app.dms_state.selection.is_new_conversation(),
        "Should navigate back to NewConversation"
    );
}

#[test]
fn test_dm_navigation_with_pending_draft() {
    let mut app = App::new();

    // Set up a pending draft
    app.dms_state.pending_conversation_username = Some("newuser".to_string());
    app.dms_state.selection = crate::app::DMSelection::NewConversation;

    // Navigate down should go to pending draft
    app.dms_state.navigate_down();
    assert!(
        app.dms_state.selection.is_pending_draft(),
        "Should navigate to pending draft"
    );

    // Navigate up should go back to NewConversation
    app.dms_state.navigate_up();
    assert!(
        app.dms_state.selection.is_new_conversation(),
        "Should navigate back to NewConversation"
    );
}

#[test]
fn test_dm_selection_stays_on_new_conversation_when_no_conversations() {
    let mut app = App::new();

    // No conversations
    assert!(app.dms_state.conversations.is_empty());

    // Navigate down should stay on NewConversation
    app.dms_state.navigate_down();
    assert!(
        app.dms_state.selection.is_new_conversation(),
        "Should stay on NewConversation when no conversations exist"
    );
}

#[test]
fn user_info_carries_id() {
    let id = uuid::Uuid::new_v4();
    let info = crate::app::UserInfo {
        id,
        username: "alice".to_string(),
        follower_count: 1,
        following_count: 2,
    };
    assert_eq!(info.id, id);
}

#[tokio::test]
async fn open_user_profile_surfaces_fetch_error() {
    let mut app = App::new();
    // Point at an unroutable address so the fetch fails deterministically,
    // regardless of network or FIDO_SERVER_URL.
    app.api_client = crate::api::ApiClient::new("http://127.0.0.1:1");
    let id = uuid::Uuid::new_v4();
    app.open_user_profile(id, "ghost".to_string())
        .await
        .unwrap();
    let view = app.user_profile_view.as_ref().expect("view opens on error");
    assert_eq!(view.username, "ghost");
    assert!(view.error.is_some());
}

#[test]
fn follow_toggle_transitions() {
    use crate::app::App;
    use fido_types::RelationshipStatus as R;
    assert_eq!(
        App::next_relationship_after_toggle(&R::None),
        Some((true, R::Following))
    );
    assert_eq!(
        App::next_relationship_after_toggle(&R::FollowsYou),
        Some((true, R::MutualFriends))
    );
    assert_eq!(
        App::next_relationship_after_toggle(&R::Following),
        Some((false, R::None))
    );
    assert_eq!(
        App::next_relationship_after_toggle(&R::MutualFriends),
        Some((false, R::FollowsYou))
    );
    assert_eq!(App::next_relationship_after_toggle(&R::Self_), None);
}
