# Design Document

## Overview

This design addresses a critical bug where hashtag filtering only shows posts created after the filter is toggled, not historical posts. The root cause is that the database schema hasn't been updated to include the hashtag tables, preventing the backfill migration from running. This design outlines the steps to fix the database schema, run the backfill migration, and verify the filtering logic works correctly for all posts.

## Architecture

### Problem Analysis

The issue has three components:

1. **Schema Mismatch**: The database file (`fido-server/fido.db`) doesn't contain the hashtag tables defined in `schema.rs`
2. **Migration Blocked**: The backfill migration tool (`fido-migrate`) cannot run because it validates the schema first
3. **Partial Functionality**: New posts work correctly because the code stores hashtags, but old posts have no associations

### Solution Architecture

```
1. Database Initialization
   ↓
2. Schema Verification
   ↓
3. Backfill Migration Execution
   ↓
4. Filtering Verification
   ↓
5. Integration Testing
```

## Components and Interfaces

### 1. Database Schema Update

The schema is already correctly defined in `fido-server/src/db/schema.rs` with all required tables:
- `hashtags`: Stores unique hashtag names
- `post_hashtags`: Junction table linking posts to hashtags
- `user_hashtag_follows`: Tracks which hashtags users follow
- `user_hashtag_activity`: Tracks user interaction metrics

**Action Required**: Ensure the database file is recreated or migrated to include these tables.

### 2. Database Initialization Check

The `Database::initialize()` method in `fido-server/src/db/connection.rs` should execute the schema SQL. We need to verify this is being called correctly.

```rust
// fido-server/src/db/connection.rs
impl Database {
    pub fn initialize(&self) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute_batch(SCHEMA)?;
        Ok(())
    }
}
```

### 3. Migration Execution Strategy

The migration tool is already implemented correctly. The execution flow:

1. **Validate Schema**: Check that all required tables exist
2. **Query Posts**: Retrieve all posts from the database
3. **Extract Hashtags**: Use the existing `extract_hashtags()` function
4. **Store Associations**: Use `HashtagRepository::store_hashtags()`
5. **Report Results**: Display statistics

**Command to run**:
```bash
cargo run --bin fido-migrate -- --database fido-server/fido.db
```

### 4. Filtering Logic Verification

The filtering logic in `PostRepository::get_posts_by_hashtag()` is correctly implemented:

```rust
pub fn get_posts_by_hashtag(&self, hashtag_name: &str, sort_order: SortOrder, limit: i32) -> Result<Vec<Post>> {
    let query = format!(
        "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id, p.reply_count
         FROM posts p
         JOIN users u ON p.author_id = u.id
         JOIN post_hashtags ph ON p.id = ph.post_id
         JOIN hashtags h ON ph.hashtag_id = h.id
         WHERE LOWER(h.name) = LOWER(?) AND p.parent_post_id IS NULL
         {}
         LIMIT ?",
        order_clause
    );
    // ...
}
```

This query correctly:
- Joins through the `post_hashtags` table
- Matches hashtags case-insensitively
- Filters out replies (parent_post_id IS NULL)
- Applies the requested sort order

### 5. TUI Filter Integration

The TUI filtering logic in `fido-tui/src/app.rs` needs to be verified to ensure it:
- Calls the correct API endpoint with hashtag parameter
- Handles the response correctly
- Updates the UI to show filtered posts

## Data Models

No changes to data models are required. The existing models are correct:

```rust
// fido-types/src/models.rs
pub struct Post {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub upvotes: i32,
    pub downvotes: i32,
    pub hashtags: Vec<String>,
    pub user_vote: Option<String>,
    pub parent_post_id: Option<Uuid>,
    pub reply_count: i32,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Acceptance Criteria Testing Prework

1.1 WHEN the database is initialized THEN the system SHALL create all required hashtag tables
  Thoughts: This is about database initialization. We can test this by initializing a fresh database and querying the sqlite_master table to verify all hashtag tables exist.
  Testable: yes - example

1.2 WHEN the backfill migration is executed THEN the system SHALL populate hashtag associations for all existing posts
  Thoughts: This is a property about the migration process. For any database with posts containing hashtags, after migration, the post_hashtags table should have entries for all those hashtags.
  Testable: yes - property

1.3 WHEN a user toggles a hashtag filter and presses Enter THEN the system SHALL display all posts containing that hashtag regardless of creation date
  Thoughts: This is a property about filtering. For any hashtag, the filtered results should include all posts (old and new) that contain that hashtag.
  Testable: yes - property

1.4 WHEN the feed is filtered by hashtag THEN the system SHALL query the post_hashtags table to find matching posts
  Thoughts: This is about implementation details - verifying the query uses the correct table. This is more of a code review item than a testable property.
  Testable: no

1.5 WHEN a new post is created with hashtags THEN the system SHALL store hashtag associations in the post_hashtags table
  Thoughts: This is a property about post creation. For any post with hashtags, after creation, the post_hashtags table should have corresponding entries.
  Testable: yes - property

2.1 WHEN the database file is inspected THEN the system SHALL contain the hashtags table with appropriate columns
  Thoughts: This is a specific check of the database schema. This is an example test.
  Testable: yes - example

2.2 WHEN the database file is inspected THEN the system SHALL contain the post_hashtags table with post_id and hashtag_id columns
  Thoughts: This is a specific check of the database schema. This is an example test.
  Testable: yes - example

2.3 WHEN the backfill migration completes THEN the system SHALL report the number of posts processed and hashtag associations created
  Thoughts: This is about the migration output. We can verify the reported statistics match the actual database state.
  Testable: yes - property

2.4 WHEN querying the post_hashtags table THEN the system SHALL return associations for posts created before the migration
  Thoughts: This is verifying the migration worked. For any post that existed before migration and contains hashtags, the post_hashtags table should have entries.
  Testable: yes - property

2.5 WHEN the application starts THEN the system SHALL use the current schema definition that includes all hashtag tables
  Thoughts: This is about initialization. We can test that a fresh database has all the required tables.
  Testable: yes - example

3.1 WHEN both friend and hashtag filters are active THEN the system SHALL show posts that match both criteria
  Thoughts: This is about combined filtering logic. For any combination of friend and hashtag filters, results should match both criteria.
  Testable: yes - property

3.2 WHEN multiple hashtags are selected THEN the system SHALL show posts containing any of the selected hashtags
  Thoughts: This is about OR logic in filtering. For any set of hashtags, results should include posts matching any of them.
  Testable: yes - property

3.3 WHEN no filters are active THEN the system SHALL show all posts in the global feed
  Thoughts: This is about the default state. When no filters are applied, all posts should be returned.
  Testable: yes - example

3.4 WHEN filters are applied THEN the system SHALL maintain the selected sort order
  Thoughts: This is about sort order persistence. For any filter and sort order combination, the results should be sorted correctly.
  Testable: yes - property

3.5 WHEN the filter modal is closed THEN the system SHALL persist the filter selections until changed by the user
  Thoughts: This is about state persistence. Filter selections should remain active across modal open/close cycles.
  Testable: yes - property

### Property Reflection

After reviewing the prework, several properties can be consolidated:
- Properties 1.2, 2.3, and 2.4 all relate to migration completeness and can be combined into one comprehensive property
- Properties 1.3 and 1.5 both test that hashtag associations work correctly and can be combined
- Properties 2.1, 2.2, and 2.5 are all schema validation examples and can be combined into one test

### Correctness Properties

Property 1: Schema initialization completeness
*For any* fresh database initialization, all required hashtag tables (hashtags, post_hashtags, user_hashtag_follows, user_hashtag_activity) should exist with the correct schema.
**Validates: Requirements 1.1, 2.1, 2.2, 2.5**

Property 2: Migration completeness and correctness
*For any* database with posts containing hashtags, after running the backfill migration, the post_hashtags table should contain associations for all hashtags in all posts, and the reported statistics should match the actual database state.
**Validates: Requirements 1.2, 2.3, 2.4**

Property 3: Hashtag filtering completeness
*For any* hashtag that appears in post content, filtering by that hashtag should return all posts (regardless of creation date) that contain the hashtag in their content.
**Validates: Requirements 1.3, 1.4**

Property 4: Post creation hashtag storage
*For any* new post created with hashtags, the post_hashtags table should immediately contain associations linking the post to all extracted hashtags.
**Validates: Requirements 1.5**

Property 5: Combined filter correctness
*For any* combination of friend and hashtag filters, the results should include only posts that match all active filter criteria.
**Validates: Requirements 3.1**

Property 6: Multiple hashtag OR logic
*For any* set of selected hashtags, the filtered results should include posts that contain at least one of the selected hashtags.
**Validates: Requirements 3.2**

Property 7: Sort order preservation
*For any* filter state and sort order, applying the filter should return results in the specified sort order.
**Validates: Requirements 3.4**

Property 8: Filter state persistence
*For any* filter selection, closing and reopening the filter modal should preserve the previously selected filters.
**Validates: Requirements 3.5**

## Error Handling

### Database Initialization Errors
- **Missing database file**: Create new database with schema
- **Schema execution failure**: Log error and exit with clear message
- **Permission errors**: Display helpful message about file permissions

### Migration Errors
- **Schema validation failure**: Display clear message that schema must be updated first
- **Post query failure**: Log error and exit
- **Hashtag storage failure**: Log error for specific post, continue with others
- **Empty database**: Display message that no posts exist to migrate

### Filtering Errors
- **Invalid hashtag name**: Return empty results
- **Database query failure**: Display error message to user
- **No results found**: Display "No posts found" message

## Testing Strategy

### Unit Testing

Unit tests will cover:
- Database schema validation
- Hashtag extraction from various post content formats
- Query construction for different filter combinations
- Sort order application

### Property-Based Testing

Property-based tests will verify:

1. **Schema Initialization Property Test**
   - Create fresh database
   - Initialize schema
   - Verify all tables exist with correct columns

2. **Migration Completeness Property Test**
   - Generate random posts with hashtags
   - Run migration
   - Verify all hashtags are in post_hashtags table
   - Verify reported statistics match actual state

3. **Filtering Completeness Property Test**
   - Generate random posts with various hashtags
   - Store them with different timestamps
   - Filter by each hashtag
   - Verify all matching posts are returned regardless of age

4. **Post Creation Storage Property Test**
   - Generate random post content with hashtags
   - Create post
   - Verify post_hashtags table has correct associations

5. **Combined Filter Property Test**
   - Generate posts from various users with various hashtags
   - Apply different filter combinations
   - Verify results match all criteria

6. **Sort Order Property Test**
   - Generate posts with various vote counts and timestamps
   - Apply different sort orders
   - Verify results are correctly ordered

### Integration Testing

Integration tests will:
- Test full flow: schema init → post creation → migration → filtering
- Test TUI filter modal interaction
- Test API endpoint responses
- Verify database state after operations

### Manual Testing Checklist

1. Delete existing database file
2. Start server (should create new database with schema)
3. Create test posts with hashtags
4. Run migration tool
5. Verify migration output shows correct counts
6. Open TUI and toggle hashtag filter
7. Verify all posts with that hashtag appear
8. Create new post with same hashtag
9. Verify it appears in filtered view
10. Test combined friend + hashtag filters

### Testing Framework

- **Property-based testing library**: `proptest` (already used in fido-migrate)
- **Unit testing**: Rust's built-in `#[test]` framework
- **Test database**: SQLite in-memory database for fast, isolated tests
- **Minimum iterations**: 100 iterations per property test

Each property-based test will be tagged with a comment referencing the design document:
```rust
// Feature: hashtag-filter-bug-fix, Property 1: Schema initialization completeness
#[test]
fn prop_schema_initialization() { ... }
```

## Implementation Plan

### Phase 1: Database Schema Fix
1. Verify schema definition in `schema.rs` is correct (already done)
2. Add database initialization verification
3. Create script to reinitialize database if needed

### Phase 2: Migration Execution
1. Verify migration tool works with correct database
2. Run migration on production database
3. Verify migration results

### Phase 3: Filtering Verification
1. Test filtering with old posts
2. Test filtering with new posts
3. Test combined filters
4. Test sort orders

### Phase 4: Integration Testing
1. Test full user flow in TUI
2. Verify API responses
3. Test edge cases

## Configuration

No configuration changes required. The existing configuration is sufficient.

## Dependencies

No new dependencies required. All necessary code and libraries are already in place:
- `fido-migrate` binary for backfill migration
- `HashtagRepository` for hashtag operations
- `PostRepository` for post queries
- `extract_hashtags()` function for parsing

## Performance Considerations

- **Migration Performance**: The migration processes posts in batches and shows progress every 100 posts
- **Query Performance**: The filtering query uses indexes on `post_hashtags.post_id` and `post_hashtags.hashtag_id`
- **Memory Usage**: Posts are processed one at a time to avoid loading entire database into memory

## Future Enhancements

- Add progress bar to migration tool
- Add rollback capability
- Add verification mode to check migration correctness
- Add incremental migration for new posts only
