# Requirements Document

## Introduction

This specification adds Vim-style H and L keybindings to the Settings modal for navigating left and right through configuration options. Currently, users can only use arrow keys (Left/Right) to change settings like color scheme and sort order. Adding H (left) and L (right) keybindings will provide a more ergonomic, keyboard-centric experience consistent with Vim navigation patterns that many developers are familiar with.

## Glossary

- **Settings Modal**: The configuration interface where users adjust preferences like color scheme and sort order
- **H Key**: Vim-style keybinding for moving left (equivalent to Left arrow)
- **L Key**: Vim-style keybinding for moving right (equivalent to Right arrow)
- **Color Scheme Setting**: The user preference for UI color theme
- **Sort Order Setting**: The user preference for how posts are sorted in the feed
- **Vim Navigation**: The keyboard navigation pattern from the Vim text editor using H/J/K/L for directional movement

## Requirements

### Requirement 1: Vim-Style Horizontal Navigation

**User Story:** As a user familiar with Vim keybindings, I want to use H and L keys to navigate left and right through settings options, so that I can configure the application without moving my hands from the home row.

#### Acceptance Criteria

1. WHEN the user presses 'H' in the Settings modal, THE TUI SHALL move to the previous option value (same behavior as Left arrow)
2. WHEN the user presses 'L' in the Settings modal, THE TUI SHALL move to the next option value (same behavior as Right arrow)
3. WHEN the user is on the first option value and presses 'H', THE TUI SHALL wrap to the last option value
4. WHEN the user is on the last option value and presses 'L', THE TUI SHALL wrap to the first option value
5. THE TUI SHALL accept both uppercase 'H' and 'L' keys for navigation (case-insensitive)

### Requirement 2: Keybinding Consistency

**User Story:** As a user, I want H/L keybindings to work alongside existing arrow key navigation, so that I can use whichever input method I prefer.

#### Acceptance Criteria

1. WHEN the user presses Left arrow or 'H' in the Settings modal, THE TUI SHALL perform identical navigation actions
2. WHEN the user presses Right arrow or 'L' in the Settings modal, THE TUI SHALL perform identical navigation actions
3. THE TUI SHALL maintain existing arrow key functionality without modification
4. THE TUI SHALL update the visual selection indicator identically for both arrow keys and H/L keys
5. THE TUI SHALL apply setting changes immediately when navigating with either arrow keys or H/L keys

### Requirement 3: Help Documentation Update

**User Story:** As a user, I want the help modal and navigation hints to show H/L keybindings, so that I can discover this navigation option.

#### Acceptance Criteria

1. WHEN the user opens the help modal, THE TUI SHALL display H/L keybindings alongside arrow key navigation in the Settings section
2. THE TUI SHALL show "H/L or ←/→: Change setting" in the Settings modal navigation bar
3. THE TUI SHALL maintain consistent formatting between arrow key and H/L key documentation
4. THE TUI SHALL list H/L keybindings in the keyboard shortcuts reference
5. THE TUI SHALL indicate that H/L keys are Vim-style alternatives to arrow keys
