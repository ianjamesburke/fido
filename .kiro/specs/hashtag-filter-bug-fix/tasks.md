# Implementation Plan

- [ ] 1. Verify and fix database schema

  - [x] 1.1 Create database inspection utility


    - Write a small utility to inspect the current database schema
    - Check if hashtag tables exist
    - Report missing tables and columns
    - _Requirements: 2.1, 2.2_

  - [x] 1.2 Verify Database::initialize() is called correctly


    - Check server startup code to ensure initialize() is called
    - Verify SCHEMA constant includes all hashtag tables
    - Test with fresh database to confirm tables are created
    - _Requirements: 1.1, 2.5_

  - [x] 1.3 Create database reinitialization script

    - Create backup of existing database
    - Implement script to recreate database with correct schema
    - Preserve existing user and post data
    - _Requirements: 1.1_

  - [ ] 1.4 Write property test for schema initialization
    - **Property 1: Schema initialization completeness**
    - **Validates: Requirements 1.1, 2.1, 2.2, 2.5**

- [ ] 2. Execute backfill migration

  - [x] 2.1 Run migration tool on development database


    - Execute fido-migrate with --dry-run first
    - Review dry-run output for correctness
    - Run actual migration
    - _Requirements: 1.2, 2.3_


  - [x] 2.2 Verify migration results

    - Query post_hashtags table to check associations
    - Verify hashtag counts match expectations
    - Check that old posts have hashtag associations
    - _Requirements: 2.4_

  - [ ] 2.3 Write property test for migration completeness
    - **Property 2: Migration completeness and correctness**
    - **Validates: Requirements 1.2, 2.3, 2.4**

- [ ] 3. Verify filtering logic

  - [x] 3.1 Test hashtag filtering with old posts



    - Query posts by hashtag that existed before migration
    - Verify results include old posts
    - Test with multiple different hashtags
    - _Requirements: 1.3, 1.4_

  - [x] 3.2 Test hashtag filtering with new posts


    - Create new posts with hashtags
    - Verify they appear in filtered results
    - Verify hashtag associations are stored correctly
    - _Requirements: 1.5_

  - [ ] 3.3 Write property test for filtering completeness
    - **Property 3: Hashtag filtering completeness**
    - **Validates: Requirements 1.3, 1.4**

  - [ ] 3.4 Write property test for post creation storage
    - **Property 4: Post creation hashtag storage**
    - **Validates: Requirements 1.5**

- [ ] 4. Test combined filtering

  - [x] 4.1 Implement combined friend and hashtag filtering



    - Verify API supports both filters simultaneously
    - Test query logic for combined filters
    - Ensure results match both criteria
    - _Requirements: 3.1_

  - [x] 4.2 Test multiple hashtag selection



    - Verify OR logic for multiple hashtags
    - Test with various hashtag combinations
    - Ensure all matching posts are returned
    - _Requirements: 3.2_

  - [ ] 4.3 Write property test for combined filters
    - **Property 5: Combined filter correctness**
    - **Validates: Requirements 3.1**

  - [ ] 4.4 Write property test for multiple hashtag OR logic
    - **Property 6: Multiple hashtag OR logic**
    - **Validates: Requirements 3.2**

- [ ] 5. Verify sort order and state persistence

  - [x] 5.1 Test sort order with filtered posts


    - Apply different sort orders (Newest, Popular, Controversial)
    - Verify filtered results are sorted correctly
    - Test with various filter combinations
    - _Requirements: 3.4_

  - [ ] 5.2 Test filter state persistence
    - Apply filters and close modal
    - Reopen modal and verify selections are preserved
    - Test across different filter types
    - _Requirements: 3.5_

  - [ ] 5.3 Write property test for sort order preservation
    - **Property 7: Sort order preservation**
    - **Validates: Requirements 3.4**

  - [ ] 5.4 Write property test for filter state persistence
    - **Property 8: Filter state persistence**
    - **Validates: Requirements 3.5**

- [ ] 6. Integration testing

  - [x] 6.1 Test full user flow in TUI



    - Start TUI with migrated database
    - Open filter modal
    - Toggle hashtag filter
    - Verify all posts with hashtag appear
    - Test with multiple hashtags
    - _Requirements: 1.3, 3.1, 3.2_

  - [x] 6.2 Test API endpoints


    - Test GET /posts?hashtag=name
    - Test with various hashtag names
    - Verify response includes old and new posts
    - Test error cases (invalid hashtag, no results)
    - _Requirements: 1.3, 1.4_

  - [x] 6.3 Test edge cases


    - Test with posts containing no hashtags
    - Test with hashtags in different formats (#rust, #Rust, #RUST)
    - Test with special characters in hashtags
    - Test with very long hashtag names
    - _Requirements: 1.3_

- [ ] 7. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Documentation and cleanup

  - [x] 8.1 Document migration process


    - Create migration guide for production deployment
    - Document backup and rollback procedures
    - Add troubleshooting section
    - _Requirements: 1.2, 2.3_

  - [x] 8.2 Update user documentation


    - Document hashtag filtering feature
    - Add examples of filter usage
    - Document keyboard shortcuts
    - _Requirements: 1.3, 3.1, 3.2_

  - [x] 8.3 Clean up debug code



    - Remove any temporary logging
    - Clean up test data
    - Verify production readiness
    - _Requirements: All_
