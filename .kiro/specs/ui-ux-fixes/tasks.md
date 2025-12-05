# Implementation Plan

- [x] 1. Refactor Bio Editor Modal
  - Implement multi-line text input with proper cursor positioning in the bio editor
  - Update `ProfileState` to track cursor position
  - Modify `handle_edit_bio_modal_keys` to save on Enter and close on Escape
  - Update `render_edit_bio_modal` to match Settings page styling with centered layout
  - Add character count display with color coding (green < 140, yellow < 160, red >= 160)
  - Implement specific error message parsing for authorization fai/lures (401/403)
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8_

- [x] 2. Standardize Navigation Bar Layout
  - Create two-tier navigation system separating page-specific and global actions
  - Implement `render_page_actions` function to display centered page-specific shortcuts
  - Update `render_main_screen` layout to include page-specific actions bar above global footer
  - Modify `render_global_footer` to show only global shortcuts (Tab, Logout, Help, Quit)
  - Apply consistent border styles and centering across all tabs
  - Update Posts tab to display post count and actions in centered format
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 3. Fix Escape Key Behavior with Priority System
  - Implement priority-based event handling in `handle_key_event`
  - Add modal closure checks before application exit (Help → Save Confirmation → Input Modals → App Exit)
  - Update all modal handlers to return early after processing Escape
  - Ensure Settings page shows save confirmation on Escape when changes exist
  - Test Escape key behavior across all modal states
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 4. Implement Feed Sorting Stability
  - Modify `vote_on_selected_post` to update vote counts locally without reloading feed
  - Implement optimistic UI updates for vote actions
  - Add rollback logic for failed vote requests
  - Remove automatic feed reload after voting
  - Preserve selected post index after voting
  - Ensure Ctrl+R explicitly refreshes and re-sorts the feed
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 5. Add Pull-to-Refresh Behavior
  - Add `show_refresh_prompt` boolean to `PostsState`
  - Modify `previous_post` to show refresh prompt when at first post
  - Implement second up-arrow press to trigger refresh action
  - Update `next_post` to hide refresh prompt when navigating down
  - Render refresh prompt UI at top of posts list with centered styling
  - Handle refresh trigger in main event loop
  - Position cursor on first post after refresh completes
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [x] 6. Implement Text Wrapping
  - Add text wrapping to `format_post_content` using textwrap crate
  - Implement wrapping in compose box with dynamic width calculation
  - Add text wrapping to bio editor modal
  - Implement wrapping for DM messages in conversation view
  - Update cursor positioning logic to account for wrapped lines
  - Ensure wrapping preserves hashtag and mention highlighting
  - Test wrapping behavior on terminal resize
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

- [x] 7. Add Edge Case Handling and Validation
  - Implement categorized error messages for network, auth, and validation errors
  - Add minimum terminal size check (60x20) with warning display
  - Implement empty input validation for posts and DMs with error feedback
  - Add unsaved changes check on logout from Settings tab
  - Ensure error messages are actionable with retry instructions
  - Test rapid key press handling (already handled by event loop)
  - Verify end-of-feed message displays correctly without wrapping
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8_

- [ ]* 8. Add Unit Tests for Core Functionality
  - Write tests for Escape key priority handling
  - Write tests for vote action preserving selection
  - Write tests for pull-to-refresh flow
  - Write tests for text wrapping logic
  - Write tests for empty input prevention
  - _Requirements: All requirements (validation)_

- [ ]* 9. Perform Integration Testing
  - Test complete bio editor flow (open, edit, save, cancel)
  - Test navigation bar consistency across all tabs
  - Test modal interaction flows with Escape key
  - Test voting with feed stability
  - Test pull-to-refresh interaction
  - Test text wrapping on various terminal sizes
  - Test error handling for network failures
  - _Requirements: All requirements (validation)_

- [x] 10. Implement DM Unread Indicator Management
  - Add `unread_counts` HashMap and `current_conversation_user` to `DMsState`
  - Add `read` boolean field to `DirectMessage` struct
  - Implement `open_conversation` method to mark messages as read when viewing
  - Implement `mark_conversation_as_read` API call and local state update
  - Update `render_tab_header` to display unread count badge on DMs tab
  - Clear unread count for conversation when user opens it
  - Update unread counts when new messages arrive
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 11. Resolve Keyboard Shortcut Conflicts with Input Mode
  - Add `InputMode` enum (Navigation, Typing) to track current mode
  - Add `input_mode` field to `App` struct
  - Refactor `handle_key_event` to check input mode before processing shortcuts
  - Implement mode-aware key handling: typing mode accepts all letters, navigation mode processes shortcuts
  - Change new DM conversation trigger from 'N' to Ctrl+N
  - Ensure 'u' and 'd' only trigger votes in navigation mode, not while typing
  - Implement `handle_typing_input` method for text input in typing mode
  - Switch to typing mode when compose box or message input is active
  - Switch to navigation mode on Escape or successful submission
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [x] 12. Implement Compose Box Visibility Toggle
  - Add `show_compose_box` boolean to `PostsState` (replace `show_new_post_modal`)
  - Implement `show_compose_box` method to display compose box and enter typing mode
  - Implement `hide_compose_box` method to hide compose box and return to navigation mode
  - Update 'n' key handler to show compose box instead of modal
  - Modify `render_posts_tab_with_data` to conditionally render compose box
  - Hide compose box by default when viewing Posts tab
  - Update `submit_new_post` to hide compose box on successful submission
  - Update navigation bar to show 'n: New Post' when compose box is hidden
  - Update navigation bar to show typing instructions when compose box is visible
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

- [ ]* 13. Test New DM and Input Mode Features
  - Test unread indicator clears when opening conversation
  - Test unread badge updates correctly on DMs tab
  - Test Ctrl+N triggers new conversation in DMs
  - Test 'N' can be typed in DM messages
  - Test 'u' and 'd' can be typed in post compose box
  - Test 'n' shows compose box on Posts tab
  - Test Escape hides compose box and returns to navigation
  - Test input mode switches correctly between typing and navigation
  - _Requirements: 8.1-8.5, 9.1-9.5, 10.1-10.6_

- [x] 14. Fix Cross-Platform Keyboard Input Consistency



  - Add platform detection for UI display: `cfg!(target_os = "macos")` for "Cmd" vs "Ctrl"
  - Implement `get_modifier_key_name()` function to return platform-appropriate key name
  - Update all UI shortcut displays to use `get_modifier_key_name()` instead of hardcoded "Ctrl"
  - Debug and fix Control key detection in `handle_key_event` for Ctrl+H, Ctrl+R, Ctrl+N on macOS
  - Add debug logging for Control key events to identify detection issues
  - Ensure Control modifier check is consistent across all keyboard shortcuts
  - Update `render_global_footer` to display platform-appropriate modifier key
  - Update `render_page_actions` to display platform-appropriate modifier key
  - Test all Control-based shortcuts on macOS (Ctrl+H, Ctrl+R, Ctrl+N, Ctrl+D)
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7_

- [x] 15. Fix Reply Modal Submission Consistency



  - Update `handle_reply_modal_keys` to submit on plain Enter (remove Ctrl requirement)
  - Remove Ctrl+Enter handling from reply modal key handler
  - Update reply modal footer text to show "Enter: Submit | Esc: Cancel"
  - Verify new post composer uses Enter (not Ctrl+Enter)
  - Verify DM message input uses Enter (not Ctrl+Enter)
  - Ensure consistent submission behavior across all composer modals
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5_

- [x] 16. Improve DMs Page UX



  - Initialize `DMsState.selected_conversation` to `Some(0)` instead of `None`
  - Update `load_conversations` to set cursor to first conversation when conversations load
  - Add top padding to DMs tab layout (1 line above header)
  - Increase message input box height from 3 to 6 lines in layout constraints
  - Update `render_message_input` to properly display 4-5 lines of text
  - Ensure cursor starts on first conversation without requiring down-arrow presses
  - Add vertical spacing between header and conversation list
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5_

- [x] 17. Implement Scroll Behavior Boundaries



  - Add `at_end_of_feed`, `last_scroll_time`, `scroll_velocity_threshold`, and `trigger_refresh` fields to `PostsState`
  - Modify `next_post` to stop at last post without wrapping to top
  - Modify `previous_post` to stop at first post (or show refresh prompt) without wrapping to bottom
  - Implement `should_trigger_refresh` method with scroll velocity detection
  - Add scroll velocity threshold configuration (default 100ms)
  - Track scroll timing with `Instant` to calculate velocity
  - Ignore pull-to-refresh if scroll velocity exceeds threshold (rapid scrolling)
  - Render "End of Feed" indicator when user reaches bottom of posts
  - Test scroll boundaries on Posts page and My Posts page
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 14.5, 14.6_

- [x] 18. Test New Cross-Platform and UX Features
  - Test Ctrl+H opens help on macOS
  - Test Ctrl+R refreshes feed on macOS
  - Test Ctrl+N opens new conversation on macOS
  - Test UI displays "Cmd" on macOS and "Ctrl" on other platforms
  - Test reply modal submits on Enter (not Ctrl+Enter)
  - Test DMs cursor starts on first conversation
  - Test DMs page has proper padding and spacing
  - Test message input box displays 4-5 lines
  - Test scrolling stops at bottom without wrapping
  - Test pull-to-refresh ignores rapid trackpad scrolling
  - Test "End of Feed" indicator displays at bottom
  - _Requirements: 11.1-11.7, 12.1-12.5, 13.1-13.5, 14.1-14.6_

- [x] 19. Implement Comment Count Refresh After Reply



  - Add `ConversationState` struct with `viewing_post_id` and `reply_submitted` fields
  - Add `conversation_state` field to `App` struct
  - Add `comment_count` field to `Post` struct if not already present
  - Modify `open_conversation` to store the post ID being viewed in `conversation_state`
  - Update `submit_reply` to increment the comment count in the cached post immediately after successful submission
  - Implement optimistic update pattern: find post in `posts_state.posts` by ID and increment `comment_count`
  - Add error handling to prevent comment count increment if reply submission fails
  - Ensure comment count updates are visible when returning to Posts tab
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 15.5_

- [x] 20. Test Comment Count Refresh Feature



  - Test comment count increments immediately after submitting a reply
  - Test comment count remains unchanged if reply submission fails
  - Test comment count updates are visible when navigating back to Posts tab
  - Test multiple replies increment comment count correctly
  - Test comment count persists across tab switches (until next feed reload)
  - _Requirements: 15.1-15.5_
