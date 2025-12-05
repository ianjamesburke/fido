# Requirements Document

## Introduction

This specification addresses a critical bug in the Fido TUI modal system where the thread modal disappears when the reply composer modal opens on top of it. The thread modal should remain visible in the background (like how the profile modal works), but currently it vanishes and only reappears after the first spacebar press in the reply composer. The goal is to fix this bug by understanding how the profile modal layering works correctly and applying the same pattern to the thread + reply composer interaction, while also cleaning up dead code from previous hacky fixes.

## Glossary

- **Thread Modal**: The modal showing a post's thread with all replies, opened by pressing Enter on a post in the feed
- **Reply Composer Modal**: The text input modal for composing a reply to a post, opened by pressing 'R' while viewing a thread
- **Profile Modal**: The modal showing user profile information, opened by pressing 'P' on a post (this works correctly as a reference)
- **Profile Tab**: The main tab for viewing your own profile (different from the profile modal)
- **Modal Layering**: The visual stacking of modals where one modal appears on top of another
- **Keyboard Priority**: Only the topmost modal should receive keyboard input
- **Spacebar Bug**: The first spacebar press in reply composer causes the thread modal to appear instead of inserting a space (likely due to async re-render triggering state mismatch)

## Requirements

### Requirement 1: Thread Modal Visibility During Reply

**User Story:** As a user, I want the thread modal to remain visible in the background when I open the reply composer, so that I can see the context of what I'm replying to.

#### Acceptance Criteria

1. WHEN the user presses Enter on a post in the global feed, THE TUI SHALL open the thread modal displaying the post and its replies
2. WHEN the user presses 'R' while viewing a thread modal, THE TUI SHALL open the reply composer modal on top of the thread modal
3. WHILE the reply composer modal is open, THE TUI SHALL continue rendering the thread modal in the background
4. WHEN the user closes the reply composer modal, THE TUI SHALL return keyboard focus to the thread modal
5. THE TUI SHALL maintain the thread modal's state (scroll position, selected reply) while the reply composer is open

### Requirement 2: Spacebar Input Handling

**User Story:** As a user, I want the spacebar to insert a space character immediately on the first press in the reply composer, so that I can type naturally without workarounds.

#### Acceptance Criteria

1. WHEN the user presses spacebar in the reply composer modal, THE TUI SHALL insert a space character in the text input
2. THE TUI SHALL handle the first spacebar press identically to all subsequent spacebar presses
3. THE TUI SHALL not consume the spacebar press for any purpose other than text input
4. THE TUI SHALL not trigger any state changes in the thread modal when spacebar is pressed in the reply composer
5. THE TUI SHALL not cause the thread modal to appear or change visibility when spacebar is pressed

### Requirement 3: Modal Rendering Order Fix

**User Story:** As a developer, I want the modal rendering logic to match the working profile modal pattern, so that the thread modal always renders when it should be visible.

#### Acceptance Criteria

1. THE TUI SHALL render the thread modal whenever viewing_post_detail is true and show_full_post_modal is true, regardless of whether the composer is open
2. THE TUI SHALL render the reply composer modal after the thread modal so it appears on top (current order is correct)
3. THE TUI SHALL not skip rendering the thread modal when the composer is open
4. THE TUI SHALL investigate why the dimmed background conditional (lines 173-176 in tabs.rs) may be causing the thread modal to not render properly
5. THE TUI SHALL ensure the thread modal renders consistently like the profile modal does (profile modal renders after composer and works perfectly)

### Requirement 4: Debug Logging Infrastructure

**User Story:** As a developer, I want comprehensive logging of modal operations, so that I can diagnose rendering and state issues.

#### Acceptance Criteria

1. THE TUI SHALL write debug logs to a file that is cleared on each application run
2. THE TUI SHALL log modal rendering decisions (which modals are being rendered and why)
3. THE TUI SHALL log keyboard events and which modal receives them
4. THE TUI SHALL log state changes related to viewing_post_detail and composer_state
5. THE TUI SHALL make logs easily readable with clear formatting and timestamps

### Requirement 5: Dead Code Cleanup

**User Story:** As a developer, I want dead code and hacky fixes removed from the modal system, so that the codebase is clean and maintainable.

#### Acceptance Criteria

1. THE TUI SHALL remove all unused modal-related functions and state variables
2. THE TUI SHALL remove commented-out modal code from previous fix attempts
3. THE TUI SHALL consolidate duplicate modal rendering logic where appropriate
4. THE TUI SHALL eliminate redundant modal state flags
5. THE TUI SHALL document the final modal rendering pattern with clear comments
