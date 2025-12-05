# Implementation Plan

- [x] 1. Create Database Schema for Friendships



  - Create `friendships` table with user_id, friend_id, and created_at fields
  - Add composite primary key on (user_id, friend_id)
  - Add foreign key constraints to users table with CASCADE delete
  - Create indexes on user_id and friend_id columns
  - Write database migration script
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

- [x] 2. Implement Add Friend API Endpoint



  - Create `POST /friends/:username` endpoint
  - Implement username validation (trim whitespace, check not empty)
  - Implement case-insensitive username lookup
  - Validate user exists in database (return 404 if not found)
  - Prevent adding self as friend (return 400 error)
  - Use INSERT OR IGNORE to prevent duplicate friendships
  - Add authentication middleware
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 3. Implement Get Friends API Endpoint



  - Create `GET /friends` endpoint
  - Join friendships with users table to get friend details
  - Include username, display_name, friend_count, and added_at in response
  - Sort friends by created_at descending (most recent first)
  - Return empty array when user has no friends
  - Add authentication middleware
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 4. Implement Remove Friend API Endpoint



  - Create `DELETE /friends/:id` endpoint
  - Delete friendship record by user_id and friend_id
  - Return 404 if friendship does not exist
  - Return 200 on successful removal
  - Add authentication middleware
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 5. Add Friends State Management to TUI




  - Add `FriendsState` struct with friends list, modal state, and input state
  - Add `FriendInfo` struct for friend data
  - Implement `load_friends` method to fetch friends from API
  - Implement `add_friend` method with error handling
  - Implement `remove_friend` method
  - Initialize friends state in app constructor
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_


- [x] 6. Implement Friends Modal UI



  - Create `render_friends_modal` function with centered layout
  - Implement `render_friends_list` to display friends with usernames and friend counts
  - Add "Add Friend" option at bottom of friends list
  - Display formatted timestamps for when friends were added
  - Add footer with keyboard shortcut instructions
  - Style selected friend with highlighting
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [x] 7. Implement Add Friend Input UI

  - Create `render_add_friend_input` function
  - Display username input field with @ prefix
  - Show error message if username not found
  - Add title "Enter username to add as friend"
  - Add footer with "Enter: Add Friend | Esc: Cancel" instructions
  - Clear input field after successful addition
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [x] 8. Implement Friends Modal Navigation


  - Add 'F' key handler to open friends modal on Profile page
  - Implement up/down arrow navigation in friends list
  - Implement Enter key to select "Add Friend" option
  - Implement 'X' key to remove selected friend
  - Implement Escape key to close modal
  - Load friends list when modal opens
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 9. Implement Add Friend Input Handling

  - Create `handle_add_friend_input_keys` function
  - Handle character input to build username string
  - Handle Backspace to delete characters
  - Handle Enter to submit username and add friend
  - Handle Escape to cancel and return to friends list
  - Display error message if username not found
  - Clear input and close on successful addition
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [x] 10. Implement Remove Friend Confirmation


  - Create confirmation modal for friend removal
  - Display "Remove @username from friends?" message
  - Handle Enter to confirm removal
  - Handle Escape to cancel removal
  - Update friends list after successful removal
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 11. Integrate Friends into DM User List

  - Modify `load_dm_users` to fetch friends list
  - Separate friends from non-friends in user list
  - Sort friends by most recent friendship (added_at descending)
  - Place friends at top of DM user list
  - Add visual separator between friends and other users
  - Track friends count for separator positioning
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [x] 12. Implement DM Error Handling


  - Add `DMErrorState` struct to track error modal state
  - Modify `send_dm_to_username` to catch 404 errors
  - Display error modal with "User '@username' not found" message
  - Add "Add them as a friend first?" suggestion to error message
  - Store failed username for pre-filling add friend input
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_


- [x] 13. Implement DM Error Modal UI


  - Create `render_dm_error_modal` function with centered layout
  - Display error message with text wrapping
  - Add footer with "Enter: Add Friend | Esc: Cancel" instructions
  - Style modal with red border for error indication
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [x] 14. Implement DM Error Modal Navigation

  - Create `handle_dm_error_modal_keys` function
  - Handle Enter to open add friend interface with pre-filled username
  - Handle Escape to close error modal
  - Clear failed username after modal closes
  - _Requirements: 8.3, 8.4, 8.5_

- [x] 15. Add Username Validation Helper

  - Create `validate_username` helper function
  - Check username exists in database (case-insensitive)
  - Trim whitespace from input
  - Return 404 error if user not found
  - Return 400 error if username is empty
  - Use in both add friend and DM endpoints
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_

- [x] 16. Add Unit Tests for Friends System



  - Test username validation (empty, whitespace, case-insensitive)
  - Test duplicate friendship prevention
  - Test self-friendship prevention
  - Test friends list sorting by added_at
  - Test DM user list prioritization
  - _Requirements: All requirements (validation)_

- [x] 17. Perform Integration Testing



  - Test add friend flow with valid username
  - Test add friend flow with invalid username
  - Test remove friend flow
  - Test friends list retrieval
  - Test friends appear first in DM user list
  - Test DM error modal with friend suggestion
  - Test add friend from DM error modal
  - _Requirements: All requirements (validation)_

