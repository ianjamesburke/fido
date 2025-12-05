# Requirements Document

## Introduction

This specification defines a friends system for the Fido terminal application. The system enables users to maintain a list of friends for easier direct messaging and content discovery. The implementation includes database storage for friendships, API endpoints for friend management, and UI components for adding and managing friends. The design prioritizes simplicity with direct username entry rather than complex search algorithms.

## Glossary

- **Friend**: A user that another user has added to their friends list
- **Friendship**: A unidirectional relationship where one user has added another as a friend
- **Friends List**: The collection of users that a user has added as friends
- **Friends Modal**: A modal interface for viewing and managing friends
- **Username Entry**: Direct input of a username to add as a friend (no fuzzy search)
- **DM Priority**: Showing friends first in DM conversation selection

## Requirements

### Requirement 1: Friendship Data Model

**User Story:** As a developer, I want a simple data model for friendships, so that the system can efficiently store and query friend relationships.

#### Acceptance Criteria

1. THE System SHALL store friendships in a dedicated table with user_id and friend_id pairs
2. THE System SHALL record the timestamp when each friendship was created
3. THE System SHALL use foreign key constraints to ensure both users exist
4. THE System SHALL support unidirectional friendships (user A can add user B without B adding A)
5. THE System SHALL prevent duplicate friendship entries for the same user pair
6. THE System SHALL generate unique identifiers using composite primary keys

### Requirement 2: Add Friend by Username

**User Story:** As a user, I want to add friends by entering their username directly, so that I can quickly build my friends list without complex search interfaces.

#### Acceptance Criteria

1. WHEN a user enters a username to add as a friend, THE System SHALL validate that the username exists
2. WHEN the username exists, THE System SHALL create a friendship record
3. WHEN the username does not exist, THE System SHALL return an error message indicating the user was not found
4. THE System SHALL prevent users from adding themselves as friends
5. THE System SHALL prevent duplicate friendships (adding the same user twice)
6. THE System SHALL perform case-insensitive username matching


### Requirement 3: Friends List Retrieval

**User Story:** As a user, I want to view my friends list, so that I can see who I've added and manage my connections.

#### Acceptance Criteria

1. WHEN a user requests their friends list, THE System SHALL return all users they have added as friends
2. THE System SHALL include friend usernames, display names, and friend counts in the response
3. THE System SHALL sort friends by the date they were added (most recent first)
4. WHEN a user has no friends, THE System SHALL return an empty list without errors
5. THE System SHALL include the timestamp when each friend was added

### Requirement 4: Remove Friend

**User Story:** As a user, I want to remove friends from my list, so that I can manage my connections over time.

#### Acceptance Criteria

1. WHEN a user removes a friend, THE System SHALL delete the friendship record
2. WHEN the friendship does not exist, THE System SHALL return a 404 error
3. THE System SHALL allow removal by friend user ID
4. THE System SHALL not affect the friend's ability to have the user in their own friends list
5. THE System SHALL return success status after successful removal

### Requirement 5: Friends Modal UI

**User Story:** As a user, I want a modal interface to view and manage my friends, so that I can easily add or remove friends from the profile page.

#### Acceptance Criteria

1. WHEN the user presses 'F' on the Profile page, THE TUI SHALL display the friends modal
2. WHEN the friends modal is open, THE TUI SHALL display the current friends list
3. WHEN the friends modal is open, THE TUI SHALL provide an "Add Friend" option
4. WHEN the user selects "Add Friend", THE TUI SHALL display a username input field
5. WHEN the user presses Escape in the friends modal, THE TUI SHALL close the modal
6. THE TUI SHALL display friend usernames and friend counts in the list

### Requirement 6: Add Friend UI Flow

**User Story:** As a user, I want to add friends through a simple input interface, so that I can quickly expand my friends list.

#### Acceptance Criteria

1. WHEN the user selects "Add Friend", THE TUI SHALL display a text input field for username entry
2. WHEN the user enters a username and presses Enter, THE TUI SHALL attempt to add that user as a friend
3. WHEN the username is valid and exists, THE TUI SHALL add the friend and update the friends list
4. WHEN the username does not exist, THE TUI SHALL display an error message "User '@username' not found"
5. WHEN the user presses Escape in the add friend input, THE TUI SHALL return to the friends list
6. THE TUI SHALL clear the input field after successful friend addition

### Requirement 7: Remove Friend UI Flow

**User Story:** As a user, I want to remove friends from the modal, so that I can manage my friends list without leaving the interface.

#### Acceptance Criteria

1. WHEN the user presses 'X' on a selected friend in the friends modal, THE TUI SHALL prompt for confirmation
2. WHEN the user confirms removal, THE TUI SHALL remove the friend and update the friends list
3. WHEN the user cancels removal, THE TUI SHALL return to the friends list without changes
4. THE TUI SHALL display a confirmation message "Remove @username from friends?"
5. THE TUI SHALL update the friends list immediately after successful removal

### Requirement 8: DM Error Handling with Friend Suggestion

**User Story:** As a user, I want helpful error messages when trying to DM non-existent users, so that I can take corrective action.

#### Acceptance Criteria

1. WHEN a user attempts to send a DM to a non-existent username, THE TUI SHALL display an error modal
2. THE TUI SHALL display the message "User '@username' not found. Add them as a friend first?"
3. WHEN the user presses Enter in the error modal, THE TUI SHALL open the add friend interface
4. WHEN the user presses Escape in the error modal, THE TUI SHALL close the modal and return to DMs
5. THE TUI SHALL pre-fill the username in the add friend interface if opened from the error modal

### Requirement 9: Friends-First DM Conversation Selection

**User Story:** As a user, I want to see my friends first when starting a new DM conversation, so that I can quickly message people I know.

#### Acceptance Criteria

1. WHEN the user opens the new conversation modal, THE TUI SHALL display friends at the top of the list
2. WHEN displaying friends, THE TUI SHALL show a visual separator between friends and other users
3. WHEN a user has no friends, THE TUI SHALL display all users without a separator
4. THE TUI SHALL sort friends by most recent friendship (newest first)
5. THE TUI SHALL allow selection of any user (friend or non-friend) for DM conversation

### Requirement 10: Username Validation

**User Story:** As a developer, I want robust username validation, so that the system handles invalid inputs gracefully.

#### Acceptance Criteria

1. WHEN validating a username, THE System SHALL check that the username exists in the database
2. WHEN a username does not exist, THE System SHALL return a 404 error with a clear message
3. THE System SHALL perform case-insensitive username lookups
4. THE System SHALL trim whitespace from username inputs
5. THE System SHALL reject empty usernames with a 400 error

