# Design Document

## Overview

This design addresses a critical bug where the thread modal disappears when the reply composer opens. Through code analysis, we've identified that the issue is related to the conditional dimmed background rendering logic in `tabs.rs`. The profile modal works correctly because it renders after the composer, but the thread modal renders before the composer and may be getting skipped or cleared by the dimmed background logic.

## Architecture

### Current Modal Rendering Flow

The modal rendering happens in `fido/fido-tui/src/ui/tabs.rs`:

1. **Thread Modal** (lines 171-179) - Conditionally renders dimmed background, then renders thread modal
2. **Delete Confirmation Modal** (lines 182-189)
3. **Composer Modal** (lines 193-195)
4. **Other Modals** including **Profile Modal** (line 217-219)

### Why Profile Modal Works

The profile modal renders **after** the composer modal, so it appears on top of everything with no conditional logic interfering.

### Why Thread + Composer Fails

The thread modal renders **before** the composer. The problematic code:

```rust
if show_full_post_modal {
    if !app.composer_state.is_open() {
        render_dimmed_background(frame, area);
    }
    render_full_post_modal(frame, app, area);
}
```

## Components and Interfaces

### 1. Modal Rendering Logic Fix

**Recommended Approach**: Separate dimming from modal rendering

```rust
// Dimmed background (only when no composer)
if show_full_post_modal && !app.composer_state.is_open() {
    render_dimmed_background(frame, area);
}

// Thread modal (always render if viewing)
if show_full_post_modal {
    render_full_post_modal(frame, app, area);
}

// Composer modal
if app.composer_state.is_open() {
    render_unified_composer_modal(frame, app, area);
}
```

This ensures the thread modal always renders while still allowing conditional dimming.

### 2. Debug Logging Infrastructure

**Log File**: `fido_modal_debug.log` (cleared on each run)

**Implementation**:
- Log modal state at render start
- Log before each modal renders
- Log key events with modal state
- Include timestamps for timing analysis

### 3. State Management

**Current State** (no changes needed):
- `app.viewing_post_detail: bool`
- `app.post_detail_state.show_full_post_modal: bool`
- `app.composer_state.mode: Option<ComposerMode>`

State transitions are correct. The problem is purely in rendering logic.

## Testing Strategy

### Manual Testing Checklist

1. Open thread (Enter) - verify thread modal appears
2. Press 'R' to reply - **VERIFY thread modal stays visible**
3. Press spacebar - **VERIFY space is inserted immediately**
4. Type text - verify all input works
5. Press Escape - verify returns to thread modal
6. Compare to profile modal behavior (press 'P' in thread)

## Implementation Plan

1. Add debug logging
2. Fix modal rendering (separate dimming from rendering)
3. Test thread + composer interaction
4. Clean up dead code
5. Verify with manual testing

## Success Criteria

1. ✅ Thread modal remains visible when reply composer opens
2. ✅ First spacebar press works correctly
3. ✅ Behavior matches profile modal pattern
4. ✅ No flickering or visual glitches
