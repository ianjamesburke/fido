# Requirements Document

## Introduction

This specification addresses critical UI/UX issues in the Fido terminal application, focusing on bio editing functionality, navigation consistency, keyboard shortcut conflicts, feed behavior, text wrapping, and state synchronization. The goal is to create a polished, consistent user experience across all screens while maintaining the keyboard-driven, developer-centric philosophy of the application. Special attention is given to ensuring the UI reflects user actions immediately through optimistic updates.

## Glossary

- **TUI**: Terminal User Interface - the text-based interface rendered in the terminal
- **Bio Editor**: The modal interface for editing user profile biography text
- **Compose Box**: The input area at the top of the Posts feed for creating new posts
- **Feed**: The scrollable list of posts in the Posts tab
- **Modal**: An overlay dialog that appears on top of the main interface
- **Navigation Bar**: The bottom bar showing keyboard shortcuts for the current screen
- **Settings Page**: The configuration interface for user preferences
- **Profile Page**: The user profile view showing bio and posts
- **Escape Key Conflict**: The issue where Escape both closes modals and exits the application

## Requirements

### Requirement 1: Bio Editor Refactor

**User Story:** As a user, I want to edit my profile bio with a reliable, intuitive interface, so that I can update my profile information without errors or confusion.

#### Acceptance Criteria

1. WHEN the user presses 'e' on the Profile tab, THE Bio Editor SHALL display a multi-line text input modal with the current bio content pre-populated
2. WHILE the Bio Editor is open, THE Bio Editor SHALL display a cursor aligned correctly with the text input position
3. WHEN the user types characters in the Bio Editor, THE Bio Editor SHALL accept standard text editing operations including character insertion, deletion, and newline insertion
4. WHEN the user presses Enter in the Bio Editor, THE Bio Editor SHALL save the bio content and close the modal
5. WHEN the user presses Escape in the Bio Editor, THE Bio Editor SHALL close the modal without saving changes
6. IF the bio update request fails due to authorization, THEN THE Bio Editor SHALL display a clear error message indicating the specific authorization issue
7. THE Bio Editor SHALL enforce a maximum length of 160 characters for bio content
8. THE Bio Editor SHALL resemble the Settings page input style with consistent borders, colors, and layout

### Requirement 2: Navigation Bar Consistency

**User Story:** As a user, I want consistent navigation information across all screens, so that I can quickly understand available actions without confusion.

#### Acceptance Criteria

1. THE TUI SHALL display page-specific navigation shortcuts in a centered, boxed format on all screens
2. THE TUI SHALL display global navigation shortcuts (Tab, Shift+Tab, Logout, Help, Quit) in a separate bottom bar on all screens
3. WHEN viewing the Posts tab, THE TUI SHALL display post count, vote controls, and refresh instructions in a centered format matching the Settings page style
4. WHEN viewing the Settings page, THE TUI SHALL maintain the current centered navigation box format
5. THE TUI SHALL use consistent border styles, colors, and spacing for all navigation elements across all tabs
6. THE TUI SHALL separate page-specific actions from global navigation actions visually

### Requirement 3: Escape Key Behavior Resolution

**User Story:** As a user, I want predictable Escape key behavior, so that I can exit modals without accidentally closing the entire application.

#### Acceptance Criteria

1. WHEN a modal is open (Bio Editor, New Post, New Conversation, Save Confirmation), THE TUI SHALL close only the modal when Escape is pressed
2. WHEN no modal is open, THE TUI SHALL exit the application when Escape is pressed
3. WHEN the Settings page has unsaved changes and Escape is pressed, THE TUI SHALL display the save confirmation modal instead of exiting
4. THE TUI SHALL process Escape key events in order of priority: modal closure first, then application exit
5. THE TUI SHALL provide an alternative keyboard shortcut (Ctrl+Q or 'q') for quitting the application that works consistently regardless of modal state

### Requirement 4: Feed Sorting Stability

**User Story:** As a user, I want the post order to remain stable while browsing, so that I can vote on posts without experiencing disorienting jumps in the feed.

#### Acceptance Criteria

1. WHEN viewing posts sorted by Popular or Controversial, THE TUI SHALL maintain the current post order when the user votes on a post
2. WHEN the user votes on a post, THE TUI SHALL update the vote counts locally without reloading the entire feed
3. WHEN the user presses Ctrl+R on the Posts tab, THE TUI SHALL refresh the feed and apply the current sort order with updated data
4. THE TUI SHALL preserve the user's scroll position and selected post index after voting
5. THE TUI SHALL only re-sort posts when the user explicitly triggers a refresh action

### Requirement 5: Pull-to-Refresh Behavior

**User Story:** As a user, I want to refresh the feed by scrolling to the top, so that I can check for new posts using a familiar social media interaction pattern.

#### Acceptance Criteria

1. WHEN the user is viewing the first post in the feed, THE TUI SHALL display a "Pull to Refresh" prompt above the first post
2. WHEN the user presses the up arrow key while viewing the "Pull to Refresh" prompt, THE TUI SHALL refresh the feed and load new posts
3. WHEN the user presses the up arrow key while viewing the first post (without a refresh prompt), THE TUI SHALL display the "Pull to Refresh" prompt
4. THE TUI SHALL display the refresh prompt in a centered, visually distinct format
5. THE TUI SHALL NOT wrap the cursor to the bottom of the feed when the user presses up at the top
6. AFTER refreshing via the pull-to-refresh action, THE TUI SHALL position the cursor on the first post in the refreshed feed

### Requirement 6: Text Wrapping for Posts

**User Story:** As a user, I want long post content to wrap within the terminal width, so that I can read all post content without horizontal scrolling.

#### Acceptance Criteria

1. WHEN rendering a post with content exceeding the terminal width, THE TUI SHALL wrap the text to multiple lines within the available width
2. THE TUI SHALL preserve word boundaries when wrapping text (no mid-word breaks)
3. WHEN rendering the Compose Box input, THE TUI SHALL wrap text as the user types beyond the terminal width
4. THE TUI SHALL maintain proper indentation for wrapped lines in posts
5. THE TUI SHALL wrap text in the Bio Editor when content exceeds the modal width
6. THE TUI SHALL wrap text in DM messages when content exceeds the message area width
7. THE TUI SHALL recalculate text wrapping when the terminal is resized

### Requirement 7: Edge Case Analysis and Bug Prevention

**User Story:** As a user, I want the application to handle edge cases gracefully, so that I can use the application reliably without encountering unexpected errors.

#### Acceptance Criteria

1. WHEN the user attempts to vote on a post while offline, THE TUI SHALL display a clear error message and maintain the current UI state
2. WHEN the user attempts to edit a bio with no internet connection, THE TUI SHALL display a network error message without closing the modal
3. WHEN the terminal window is resized below minimum dimensions, THE TUI SHALL display a message indicating minimum size requirements
4. WHEN the user rapidly presses navigation keys, THE TUI SHALL process events in order without skipping or duplicating actions
5. WHEN the user switches tabs with unsaved changes in Settings, THE TUI SHALL display the save confirmation modal consistently
6. WHEN the user attempts to send an empty DM or post, THE TUI SHALL prevent submission and provide visual feedback
7. WHEN the API returns an unexpected error format, THE TUI SHALL display a generic error message instead of crashing
8. WHEN the user scrolls past the last post in the feed, THE TUI SHALL display an "End of Feed" message without wrapping to the top

### Requirement 8: DM Unread Indicator Management

**User Story:** As a user, I want unread message indicators to clear when I view conversations, so that I can accurately track which messages I haven't read yet.

#### Acceptance Criteria

1. WHEN the user navigates to the DMs tab, THE TUI SHALL display an unread indicator badge on the DMs tab if unread messages exist
2. WHEN the user opens a conversation with unread messages, THE TUI SHALL clear the unread count for that conversation
3. WHEN the user views a conversation, THE TUI SHALL mark all messages in that conversation as read
4. THE TUI SHALL update the unread indicator badge on the DMs tab to reflect the current total unread count across all conversations
5. WHEN the user navigates away from a conversation and returns, THE TUI SHALL NOT show unread indicators for messages that were already viewed

### Requirement 9: Keyboard Shortcut Conflict Resolution

**User Story:** As a user, I want to type any letter in my messages and posts without triggering command shortcuts, so that I can express myself freely without workarounds.

#### Acceptance Criteria

1. WHEN the user is typing in the DM message input field, THE TUI SHALL accept the letter 'N' as text input without triggering the new conversation command
2. WHEN the user is typing in the post compose box, THE TUI SHALL accept the letters 'u' and 'd' as text input without triggering upvote/downvote commands
3. WHEN the user wants to start a new DM conversation, THE TUI SHALL respond to Ctrl+N instead of the 'N' key alone
4. WHEN the user is not actively typing in an input field, THE TUI SHALL respond to 'u' and 'd' keys for upvote/downvote actions
5. THE TUI SHALL clearly distinguish between "typing mode" (input fields active) and "navigation mode" (browsing content)

### Requirement 10: Compose Box Visibility Toggle

**User Story:** As a user, I want the post compose box to be hidden by default and appear only when I want to create a post, so that keyboard shortcuts don't conflict with typing and the interface is less cluttered.

#### Acceptance Criteria

1. WHEN the user views the Posts tab, THE TUI SHALL hide the compose box by default
2. WHEN the user presses 'n' on the Posts tab while the compose box is hidden, THE TUI SHALL display the compose box and focus the input field
3. WHEN the compose box is visible and focused, THE TUI SHALL accept all letter keys as text input without triggering navigation or voting commands
4. WHEN the user presses Escape while the compose box is visible, THE TUI SHALL hide the compose box and return to navigation mode
5. WHEN the user successfully submits a post, THE TUI SHALL hide the compose box and return to navigation mode
6. THE TUI SHALL display a visual indicator in the navigation bar showing 'n: New Post' when the compose box is hidden

### Requirement 11: Cross-Platform Keyboard Input Consistency

**User Story:** As a user on macOS, I want Control-based keyboard shortcuts to work reliably, so that I can use the application with the same shortcuts as on other platforms.

#### Acceptance Criteria

1. WHEN the user presses Ctrl+H on macOS, THE TUI SHALL open the help modal
2. WHEN the user presses Ctrl+R on macOS, THE TUI SHALL refresh the current feed
3. WHEN the user presses Ctrl+N on macOS in the DMs tab, THE TUI SHALL open the new conversation modal
4. THE TUI SHALL detect Control key modifiers consistently across Windows, macOS, and Linux platforms
5. THE TUI SHALL display platform-appropriate key names in help text and navigation bars (show "Cmd" on macOS, "Ctrl" elsewhere)
6. THE TUI SHALL use the Control modifier for all keyboard shortcuts regardless of platform
7. WHEN displaying keyboard shortcuts in the UI, THE TUI SHALL use "Cmd" abbreviation on macOS and "Ctrl" abbreviation on other platforms

### Requirement 12: Reply Modal Submission Consistency

**User Story:** As a user, I want to submit replies using the Enter key like all other composer modals, so that I have a consistent experience across the application.

#### Acceptance Criteria

1. WHEN the user presses Enter in the reply modal, THE TUI SHALL submit the reply and close the modal
2. WHEN the user presses Escape in the reply modal, THE TUI SHALL close the modal without submitting
3. THE TUI SHALL display "Enter: Submit | Esc: Cancel" in the reply modal footer
4. THE TUI SHALL NOT require Ctrl+Enter to submit replies
5. THE TUI SHALL use the same submission behavior for reply modals, new post composer, and DM message input

### Requirement 13: DMs Page UX Improvements

**User Story:** As a user, I want the DMs page to be immediately usable when I navigate to it, so that I can quickly access my conversations without extra navigation steps.

#### Acceptance Criteria

1. WHEN the user navigates to the DMs tab, THE TUI SHALL position the cursor on the first conversation in the list
2. WHEN the DMs tab is displayed, THE TUI SHALL show 1-2 lines of vertical padding above the conversation list header
3. WHEN the user is composing a DM message, THE TUI SHALL display a message input box with a minimum height of 4-5 lines
4. THE TUI SHALL NOT require the user to press down arrow keys to reach the first conversation
5. THE TUI SHALL provide adequate visual spacing between the header and conversation list

### Requirement 14: Scroll Behavior Boundaries

**User Story:** As a user, I want scrolling to stop at natural boundaries, so that I don't experience disorienting wrap-around behavior when browsing content.

#### Acceptance Criteria

1. WHEN the user scrolls down past the last post in the feed, THE TUI SHALL stop at the last post without wrapping to the top
2. WHEN the user scrolls up from the first post, THE TUI SHALL show the pull-to-refresh prompt without wrapping to the bottom
3. WHEN the user uses trackpad scrolling on the My Posts page, THE TUI SHALL detect scroll velocity and ignore pull-to-refresh if scrolling too fast
4. THE TUI SHALL implement scroll velocity detection with a configurable threshold
5. THE TUI SHALL differentiate between intentional pull-to-refresh gestures and rapid scrolling
6. THE TUI SHALL cap scroll offset at the maximum valid position (total items minus visible items)

### Requirement 15: Comment Count Refresh After Reply

**User Story:** As a user, I want to see updated comment counts immediately after leaving a reply, so that the interface reflects my recent actions without requiring a manual refresh.

#### Acceptance Criteria

1. WHEN the user exits a threaded conversation view and returns to the Posts tab, THE TUI SHALL update the comment count for that post in the local cache
2. WHEN the user successfully submits a reply in a conversation, THE TUI SHALL increment the comment count for the corresponding post by 1
3. THE TUI SHALL update the comment count optimistically without requiring a full feed reload
4. WHEN the user navigates back to the Posts tab after replying, THE TUI SHALL display the updated comment count immediately
5. THE TUI SHALL track which post the user was viewing in the conversation to update the correct post's comment count
