# Implementation Plan

- [x] 1. Create Database Schema for Hashtags


  - Create `hashtags` table with id, name, and created_at fields
  - Create `post_hashtags` junction table with foreign keys
  - Create `user_hashtag_follows` table for follow relationships
  - Create `user_hashtag_activity` table for interaction tracking
  - Add indexes on name, post_id, hashtag_id, and user_id columns
  - Write database migration script
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

- [x] 2. Implement Hashtag Parsing and Extraction


  - Create hashtag extraction module with regex pattern `#(\w{2,})`
  - Implement `extract_hashtags` function to parse post content
  - Support letters, numbers, and underscores in hashtag names
  - Normalize hashtags to lowercase for storage
  - Remove duplicate hashtags from extraction results
  - Enforce minimum length of 2 characters
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 3. Integrate Hashtag Extraction into Post Creation


  - Modify `create_post` endpoint to extract hashtags from content
  - Implement `get_or_create_hashtag` helper function
  - Link extracted hashtags to posts in `post_hashtags` table
  - Update user activity metrics when post contains hashtags
  - Handle multiple hashtags in a single post
  - Ensure atomic transaction for post creation and hashtag linking
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [x] 4. Implement Hashtag Follow/Unfollow API


  - Create `GET /hashtags/followed` endpoint to retrieve user's followed hashtags
  - Create `POST /hashtags/follow` endpoint to follow a hashtag
  - Create `DELETE /hashtags/follow/:id` endpoint to unfollow a hashtag
  - Include post counts in followed hashtags response
  - Use INSERT OR IGNORE for duplicate follow prevention
  - Add authentication middleware to all hashtag endpoints
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 5. Implement Hashtag-Based Post Filtering


  - Create `GET /posts?hashtag=:name` endpoint for filtered posts
  - Support case-insensitive hashtag matching
  - Apply sort order parameter (newest, top, hot) to filtered results
  - Return empty array for hashtags with no posts
  - Include full post metadata in filtered results
  - Limit results to 100 posts
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 6. Implement Hashtag Search API

  - Create `GET /hashtags/search?q=:query` endpoint
  - Implement partial name matching with LIKE query
  - Include post counts in search results
  - Sort results by post count (descending)
  - Limit results to 20 hashtags
  - Support case-insensitive search
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_


- [x] 7. Implement Activity Tracking System


  - Create `increment_hashtag_activity` helper function
  - Update activity on post creation with hashtags
  - Update activity on voting for posts with hashtags
  - Update activity timestamp when viewing filtered posts
  - Use INSERT ... ON CONFLICT for atomic activity updates
  - Create `GET /hashtags/activity` endpoint for most active hashtags
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_

- [x] 8. Add Filter State Management to TUI


  - Add `FilterType` enum (All, Hashtag, User) to app state
  - Add `FilterState` struct with current filter and modal state
  - Add `FilterModalTab` enum (Hashtags, Users, All)
  - Initialize filter state in app constructor
  - Add methods to load followed hashtags from API
  - Add methods to load bookmarked users from API
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3, 6.4_

- [x] 9. Implement Filter Modal UI

  - Create `render_filter_modal` function with centered layout
  - Implement tab bar rendering with Hashtags, Users, All tabs
  - Implement hashtags tab with list of followed hashtags
  - Implement users tab with list of bookmarked users
  - Implement all tab with "Return to Global Feed" option
  - Add footer with keyboard shortcut instructions
  - Style selected tab and items with highlighting
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [x] 10. Implement Filter Modal Navigation

  - Add 'F' key handler to open filter modal on Posts page
  - Implement Tab key to cycle between modal tabs
  - Implement arrow keys and j/k for list navigation
  - Implement Enter key to apply selected filter
  - Implement 'X' key to unfollow/unbookmark selected item
  - Implement Escape key to close modal without changes
  - Update selected item index when switching tabs
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

- [x] 11. Implement Filter Display Header

  - Create `render_posts_header` function to display current filter
  - Show "[All Posts]" when no filter is active
  - Show "[#hashtag]" when hashtag filter is active
  - Show "[@username]" when user filter is active
  - Include sort order in header display
  - Update header immediately when filter changes
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 12. Implement Sort Order Cycling

  - Add `SortOrder` enum (Newest, Top, Hot) to posts state
  - Add 'S' key handler to cycle through sort orders
  - Implement `cycle_sort_order` method
  - Reload posts with new sort order after cycling
  - Display current sort order in filter header
  - Preserve sort order when changing filters
  - _Requirements: 9.1, 9.2, 9.3_


- [x] 13. Implement Sort Preference Persistence

  - Create `SortPreferences` struct with global and per-filter preferences
  - Implement `save_sort_preference` method to write to JSON file
  - Store preferences in `~/.fido/sort_preferences.json`
  - Load preferences on app startup
  - Update preferences when sort order changes
  - Maintain separate preferences for global, hashtag, and user filters
  - _Requirements: 9.4, 9.5_

- [x] 14. Integrate Filtered Post Loading

  - Implement `load_filtered_posts` method in TUI
  - Call appropriate API endpoint based on filter type
  - Pass sort order parameter to API
  - Update posts state with filtered results
  - Handle empty results gracefully
  - Show loading indicator during API call
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 15. Add Hashtag Search UI



  - Create search input modal for hashtag discovery
  - Implement real-time search as user types
  - Display search results with hashtag names and post counts
  - Allow selection of search results to follow hashtag
  - Add search results to followed hashtags list
  - Close search modal after following hashtag
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 16. Update Profile Page with Active Hashtags


  - Add "Most Active Hashtags" section to profile page
  - Call activity API endpoint to get top 5 hashtags
  - Display hashtag names with interaction counts
  - Sort by interaction count (descending)
  - Show "No activity yet" message if no hashtags
  - Update section when profile loads
  - _Requirements: 10.5_

- [x] 17. Add Unit Tests for Hashtag System


  - Test hashtag extraction regex with various inputs
  - Test get_or_create_hashtag with existing and new hashtags
  - Test activity increment logic
  - Test sort order cycling
  - Test filter state management
  - _Requirements: All requirements (validation)_

- [x] 18. Perform Integration Testing



  - Test post creation with hashtag extraction
  - Test follow/unfollow flow
  - Test filtered post retrieval with different sort orders
  - Test search functionality
  - Test activity tracking across multiple interactions
  - Test filter modal navigation and selection
  - Test sort preference persistence
  - _Requirements: All requirements (validation)_

