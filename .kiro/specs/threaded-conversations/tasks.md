# Implementation Plan: Threaded Conversations & Post Interactions

- [x] 1. Set up database schema for threaded conversations




- [x] 1.1 Add parent_post_id and reply_count columns to posts table

  - Write SQL migration to add `parent_post_id TEXT NULL` column
  - Write SQL migration to add `reply_count INTEGER DEFAULT 0` column
  - Create database index on parent_post_id for query performance
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_


- [x] 1.2 Implement database helper functions for reply operations

  - Write function to increment reply_count when reply is created
  - Write function to fetch all replies for a given post_id
  - Write function to check if post has replies
  - _Requirements: 8.3, 8.5, 9.1_

- [x] 2. Implement API endpoints for reply operations




- [x] 2.1 Create GET /posts/{post_id}/replies endpoint

  - Implement handler to fetch all replies for a post
  - Return replies with author, timestamp, content, and vote counts
  - Handle 404 error when post_id doesn't exist
  - _Requirements: 9.1, 9.3, 9.5_


- [x] 2.2 Create POST /posts/{post_id}/reply endpoint

  - Implement handler to create new reply
  - Set parent_post_id to the post being replied to
  - Increment reply_count on parent post
  - Return created reply with all metadata
  - _Requirements: 3.4, 3.5, 9.2, 9.4_

- [x] 2.3 Implement authorization helper for post ownership verification

  - Write function to verify user owns a specific post
  - Use in edit and delete endpoints
  - Return 403 Forbidden if user is not post owner
  - _Requirements: 5.5, 5.6, 6.4, 6.5_



- [x] 2.4 Create PUT /posts/{post_id} endpoint for editing posts

  - Implement handler to update post content
  - Verify post ownership before allowing update
  - Validate content is not empty and within character limit
  - Return updated post
  - _Requirements: 5.4, 5.5, 5.6_



- [x] 2.5 Create DELETE /posts/{post_id} endpoint for deleting posts

  - Implement handler to delete post
  - Verify post ownership before allowing deletion
  - Handle cascade behavior for replies (delete or orphan)
  - Return success response
  - _Requirements: 6.3, 6.4, 6.5_

- [x] 3. Add post detail state management to TUI




- [x] 3.1 Create PostDetailState struct in app.rs

  - Add fields for post, replies, selected_reply_index, loading, error
  - Add fields for reply_content, show_reply_composer
  - Add fields for edit_content, show_edit_modal
  - Add fields for show_delete_confirmation, previous_feed_position
  - _Requirements: 1.1, 1.4_


- [x] 3.2 Add post detail state to App struct

  - Add post_detail_state: Option<PostDetailState> field
  - Add viewing_post_detail: bool flag
  - Initialize to None in App::new()
  - _Requirements: 1.1_

- [x] 3.3 Implement navigation functions for post detail view


  - Write open_post_detail() function to transition from feed to detail view
  - Write close_post_detail() function to return to feed
  - Store previous feed scroll position before opening detail
  - Restore feed scroll position when closing detail
  - _Requirements: 1.1, 1.4, 1.5_

- [x] 4. Implement post detail view UI rendering




- [x] 4.1 Create render_post_detail_view() function in ui.rs

  - Render full post content without truncation
  - Display author username and timestamp
  - Display current vote counts
  - Show context-sensitive action buttons based on ownership
  - _Requirements: 1.2, 1.3, 2.1, 2.2, 2.3_


- [x] 4.2 Implement action button display logic

  - Show edit, delete, reply buttons for own posts
  - Show reply, upvote, downvote buttons for other users' posts
  - Display keyboard shortcuts for each action
  - _Requirements: 2.1, 2.2, 2.3, 2.4_


- [x] 4.3 Add keyboard event handling for post detail view

  - Handle Enter key to open post detail from feed
  - Handle ESC key to close post detail and return to feed
  - Handle 'r' key to open reply composer
  - Handle 'e' key to open edit modal (if owner)
  - Handle 'x' key to show delete confirmation (if owner)
  - Handle 'u'/'d' keys for voting on post
  - _Requirements: 1.1, 1.4, 2.4, 3.1, 5.1, 6.1_

- [x] 5. Implement reply display and navigation




- [x] 5.1 Add reply list rendering to post detail view

  - Fetch and display all replies when detail view opens
  - Show each reply with author, timestamp, content, and vote counts
  - Render reply content with Markdown formatting
  - _Requirements: 4.1, 4.2, 4.3_


- [x] 5.2 Implement reply navigation with arrow keys

  - Handle up/down arrow keys to navigate between replies
  - Highlight selected reply visually
  - Update selected_reply_index in state
  - _Requirements: 4.4, 4.5_


- [x] 5.3 Implement voting on replies

  - Handle 'u' key to upvote selected reply
  - Handle 'd' key to downvote selected reply
  - Submit vote to API endpoint
  - Update reply vote counts in UI
  - _Requirements: 4.6_

- [x] 6. Implement reply composer




- [x] 6.1 Create render_reply_composer_modal() function in ui.rs

  - Display modal with reply input area
  - Show context: "Replying to @username: [truncated post preview]"
  - Support Markdown formatting in text input
  - Display character counter (280 character limit)
  - _Requirements: 3.1, 3.2, 3.3_


- [x] 6.2 Add reply composer keyboard event handling

  - Handle character input to add to reply_content
  - Handle backspace to remove characters
  - Handle Ctrl+Enter to submit reply
  - Handle ESC to cancel and close composer
  - _Requirements: 3.4, 3.6_



- [x] 6.3 Implement reply submission logic


  - Validate reply content is not empty
  - Validate reply is within character limit
  - Submit reply to POST /posts/{post_id}/reply endpoint
  - Refresh post detail view to show new reply
  - Close reply composer on success
  - _Requirements: 3.4, 3.5_

- [x] 7. Implement post editing functionality




- [x] 7.1 Create render_edit_post_modal() function in ui.rs

  - Display modal with edit input area
  - Pre-populate editor with current post content
  - Support Markdown formatting in text input
  - Display character counter (280 character limit)
  - _Requirements: 5.1, 5.2, 5.3_


- [x] 7.2 Add edit modal keyboard event handling

  - Handle character input to modify edit_content
  - Handle backspace to remove characters
  - Handle Ctrl+Enter to save changes
  - Handle ESC to cancel and close editor
  - _Requirements: 5.4, 5.7_



- [x] 7.3 Implement post update submission logic

  - Validate edited content is not empty
  - Validate edited content is within character limit
  - Submit update to PUT /posts/{post_id} endpoint
  - Update post detail view with new content
  - Close edit modal on success
  - Handle authorization errors (403 Forbidden)
  - _Requirements: 5.4, 5.5, 5.6_

- [x] 8. Implement post deletion functionality




- [x] 8.1 Create render_delete_confirmation_modal() function in ui.rs

  - Display confirmation prompt: "Delete this post? [y/n]"
  - If post has replies, show warning with reply count
  - Handle 'y' key to confirm deletion
  - Handle 'n' or ESC key to cancel
  - _Requirements: 6.1, 6.2, 6.7_

- [x] 8.2 Implement post deletion logic

  - Submit delete request to DELETE /posts/{post_id} endpoint
  - Handle authorization errors (403 Forbidden)
  - Return to feed view on successful deletion
  - Display "Post deleted" confirmation message
  - _Requirements: 6.3, 6.4, 6.5, 6.6_

- [x] 9. Integrate reply counts into feed view




- [x] 9.1 Update feed rendering to display reply counts

  - Fetch reply_count from post data
  - Display reply count alongside vote counts (e.g., "💬 3")
  - Use visual indicator (speech bubble icon)
  - _Requirements: 7.1, 7.2, 7.4_

- [x] 9.2 Update feed after reply creation

  - Increment reply_count locally when reply is created
  - Update feed display without full reload
  - _Requirements: 7.3_

- [x] 10. Implement error handling and loading states



- [x] 10.1 Add loading indicators for async operations

  - Show loading spinner when fetching post detail
  - Show loading spinner when fetching replies
  - Show loading spinner when submitting reply/edit/delete
  - _Requirements: 10.4_

- [x] 10.2 Implement error display for post detail operations

  - Display error messages for network failures
  - Display error messages for authorization failures (403)
  - Display "Post deleted" placeholder for 404 errors
  - Provide retry option for network errors
  - _Requirements: 10.1, 10.2, 10.3, 10.5_

- [x] 10.3 Add error categorization for user-friendly messages

  - Categorize 404 errors as "Post not found"
  - Categorize 403 errors as "Authorization Error"
  - Categorize network errors as "Network Error"
  - Follow existing error handling pattern
  - _Requirements: 10.1, 10.2, 10.3, 10.5_

- [ ]* 11. Write tests for threaded conversations feature
- [ ]* 11.1 Write unit tests for database operations
  - Test parent_post_id relationship creation
  - Test reply_count increment/decrement
  - Test reply fetching by parent_post_id
  - Test index performance
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ]* 11.2 Write unit tests for API endpoints
  - Test GET /posts/{post_id}/replies with valid and invalid post_id
  - Test POST /posts/{post_id}/reply with valid and invalid data
  - Test PUT /posts/{post_id} with owner and non-owner
  - Test DELETE /posts/{post_id} with owner and non-owner
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [ ]* 11.3 Write integration tests for full reply flow
  - Test reply creation from TUI to API to database
  - Test post editing with authorization
  - Test post deletion with replies
  - Test navigation state preservation
  - _Requirements: 3.4, 5.4, 6.3, 1.4_

- [ ]* 11.4 Perform manual testing of UI flows
  - Test reply creation on own and other users' posts
  - Test post editing and cancellation
  - Test post deletion with and without replies
  - Test navigation between feed and detail views
  - Test error scenarios (network failures, authorization)
  - _Requirements: All requirements_
