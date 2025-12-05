# Implementation Plan

- [ ] 1. Add debug logging infrastructure
  - Create log helper functions (log_modal_state, log_key_event, append_to_log, clear_debug_log)
  - Add log file path constant: `fido_modal_debug.log`
  - Call clear_debug_log() on application startup in main.rs
  - Add logging to render_posts_tab function in tabs.rs (log at start and before each modal render)
  - Add logging to key event handler in main.rs
  - Test that logs are being written correctly
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 2. Fix thread modal rendering logic
  - Locate the problematic code in fido/fido-tui/src/ui/tabs.rs (lines 171-179)
  - Separate dimmed background rendering from thread modal rendering
  - Change: Move `render_dimmed_background` call outside the thread modal rendering block
  - Ensure dimmed background only renders when `show_full_post_modal && !app.composer_state.is_open()`
  - Ensure thread modal always renders when `show_full_post_modal` is true, regardless of composer state
  - _Requirements: 1.1, 1.2, 1.3, 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ] 3. Test thread + composer interaction
  - Run the application and open a thread (press Enter on a post)
  - Verify thread modal appears with dimmed background
  - Press 'R' to open reply composer
  - **VERIFY**: Thread modal remains visible in background
  - **VERIFY**: Composer appears on top of thread modal
  - Press spacebar in composer
  - **VERIFY**: Space character is inserted immediately (not consumed)
  - Type additional text including multiple spaces
  - **VERIFY**: All keystrokes work correctly
  - Press Escape to close composer
  - **VERIFY**: Thread modal still visible and has keyboard focus
  - _Requirements: 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 4. Compare with profile modal behavior
  - Open a thread (press Enter on a post)
  - Press 'P' to open profile modal
  - **VERIFY**: Profile modal appears on top, thread visible in background
  - Press Escape to close profile modal
  - **VERIFY**: Returns to thread modal
  - Open composer ('R') and verify it behaves the same way as profile modal
  - _Requirements: 1.3, 3.5_

- [ ] 5. Analyze debug logs
  - Reproduce the thread + composer flow
  - Open fido_modal_debug.log
  - Verify "Rendering thread modal" appears even when composer_open=true
  - Check for any timing issues or state mismatches
  - Verify keyboard events are logged correctly
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [ ] 6. Clean up dead code
  - Search for unused modal-related functions in fido/fido-tui/src/ui/modals/
  - Search for commented-out modal code (grep for "// " in modal files)
  - Remove deprecated functions (e.g., render_edit_bio_modal marked as DEPRECATED)
  - Remove redundant state flags if any are found
  - Add documentation comments to the fixed modal rendering logic explaining the pattern
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [ ] 7. Final verification
  - Run through complete manual testing checklist from design document
  - Test all modal combinations: thread alone, thread + composer, thread + profile, thread + delete confirmation
  - Verify no flickering or visual glitches
  - Verify spacebar works on first press in all text input modals
  - Verify keyboard focus returns correctly when closing modals
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ] 8. Optional: Reduce logging verbosity
  - If logging is too verbose for production use, add a debug flag
  - Make logging conditional on the debug flag
  - Or remove logging entirely if no longer needed
  - _Requirements: 4.5_
