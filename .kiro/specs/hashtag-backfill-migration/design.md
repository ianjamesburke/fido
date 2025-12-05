# Design Document

## Overview

This design document outlines the implementation of a one-time data migration utility to backfill hashtag data for existing posts in the Fido database. The migration script will be a standalone Rust binary that reads all posts from the database, extracts hashtags from their content using the existing hashtag extraction logic, and populates the hashtag-related tables (`hashtags` and `post_hashtags`).

The migration is necessary because the hashtag storage system was implemented after some posts were already created. These older posts have hashtags in their content but lack corresponding database records, preventing hashtag filtering from working on them.

## Architecture

### Component Structure

The migration will be implemented as a standalone binary within the Fido workspace:

```
fido/
├── fido-server/
├── fido-tui/
├── fido-types/
└── fido-migrate/        # New migration utility crate
    ├── Cargo.toml
    └── src/
        └── main.rs      # Migration script entry point
```

### Dependencies

The migration utility will reuse existing Fido components:
- `fido-server` crate for database access and repository logic
- Existing `HashtagRepository` for hashtag storage operations
- Existing `extract_hashtags` function from `hashtag` module
- `clap` for command-line argument parsing
- `anyhow` for error handling

## Components and Interfaces

### Migration Script (`fido-migrate/src/main.rs`)

The main migration script will:

1. **Parse command-line arguments**
   - Database path (optional, defaults to `./fido.db`)
   - Dry-run flag (optional, shows what would be done without making changes)
   - Confirmation flag (optional, skips interactive prompt)

2. **Connect to database**
   - Use existing `Database` struct from `fido-server`
   - Validate database schema exists

3. **Execute migration**
   - Query all posts from the database
   - For each post, extract hashtags and store them
   - Track statistics (posts processed, hashtags created, errors)

4. **Report results**
   - Display summary of migration results
   - Log any errors encountered

### Key Functions

```rust
/// Main entry point for migration
fn main() -> Result<()>

/// Execute the migration with given configuration
fn run_migration(db_path: &str, dry_run: bool) -> Result<MigrationStats>

/// Process a single post and store its hashtags
fn process_post(
    post_id: &Uuid,
    content: &str,
    hashtag_repo: &HashtagRepository
) -> Result<usize>

/// Display migration statistics
fn display_stats(stats: &MigrationStats)
```

## Data Models

### MigrationStats

```rust
struct MigrationStats {
    posts_processed: usize,
    posts_with_hashtags: usize,
    total_hashtags_created: usize,
    unique_hashtags: usize,
    errors: Vec<String>,
}
```

### Command-Line Arguments

```rust
#[derive(Parser)]
struct Args {
    /// Path to the SQLite database file
    #[arg(short, long, default_value = "./fido.db")]
    database: String,
    
    /// Perform a dry run without making changes
    #[arg(short = 'n', long)]
    dry_run: bool,
    
    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    yes: bool,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property Reflection

After analyzing the acceptance criteria, several properties can be consolidated:
- Properties 1.3 and 2.1 (idempotent hashtag creation and duplicate prevention) are subsumed by the comprehensive idempotency property 2.4
- Properties 1.1 and 1.5 (reading all posts and reporting counts) can be combined - verifying accurate counts ensures all posts were processed
- Properties 3.1 and 3.2 are simple CLI examples, not universal properties

### Correctness Properties

Property 1: Migration completeness and accuracy
*For any* database state with posts, after running the migration, the reported statistics should match the actual database state: the number of posts processed should equal the total posts in the database, and the number of hashtag associations created should equal the sum of hashtags extracted from all post content.
**Validates: Requirements 1.1, 1.5**

Property 2: Extraction consistency
*For any* post content, the hashtags extracted by the migration script should be identical to the hashtags extracted by the existing `extract_hashtags` function used throughout the application.
**Validates: Requirements 1.2**

Property 3: Association correctness
*For any* post with hashtags, after migration, querying the `post_hashtags` table should return exactly the hashtags that were extracted from that post's content, with correct post_id and hashtag_id linkages.
**Validates: Requirements 1.4**

Property 4: Migration idempotency
*For any* database state, running the migration multiple times should produce the same final database state as running it once - no duplicate hashtags, no duplicate associations, and identical counts.
**Validates: Requirements 1.3, 2.1, 2.4**

Property 5: Robustness to edge cases
*For any* post content including edge cases (empty strings, special characters, malformed hashtags, very long content), the migration should complete successfully without crashing, extracting only valid hashtags.
**Validates: Requirements 2.3**

## Error Handling

### Error Categories

1. **Database Connection Errors**
   - Cannot open database file
   - Database schema is invalid or missing
   - Action: Exit with clear error message

2. **Database Query Errors**
   - Failed to read posts
   - Failed to insert hashtags
   - Action: Log error, continue with remaining posts, report errors in summary

3. **Data Validation Errors**
   - Invalid UUID in database
   - Malformed post content
   - Action: Log warning, skip problematic post, continue migration

### Error Recovery Strategy

The migration will use a "best effort" approach:
- Continue processing remaining posts if individual posts fail
- Collect all errors and display them in the final summary
- Use database transactions per post to ensure partial failures don't corrupt data
- Provide detailed error messages with post IDs for debugging

## Testing Strategy

### Unit Testing

Unit tests will cover:
- Command-line argument parsing with various inputs
- Statistics tracking and reporting
- Error message formatting
- Edge cases in post content (empty, very long, special characters)

### Property-Based Testing

Property-based tests will verify the correctness properties defined above:

1. **Completeness Property Test**
   - Generate random database with N posts containing hashtags
   - Run migration
   - Verify reported count equals N and hashtag associations match extracted hashtags

2. **Extraction Consistency Property Test**
   - Generate random post content with various hashtag patterns
   - Extract using migration logic and existing `extract_hashtags` function
   - Verify both produce identical results

3. **Association Correctness Property Test**
   - Generate random posts with hashtags
   - Run migration
   - For each post, verify database associations match extracted hashtags

4. **Idempotency Property Test**
   - Generate random database state
   - Run migration twice
   - Verify database state is identical after first and second run
   - Verify no duplicate hashtags or associations exist

5. **Robustness Property Test**
   - Generate random post content including edge cases
   - Run migration
   - Verify migration completes without panicking
   - Verify only valid hashtags are extracted

### Integration Testing

Integration tests will:
- Test migration against a real SQLite database
- Verify database schema remains valid after migration
- Test dry-run mode produces no changes
- Verify migration works with empty database
- Test migration with database containing existing hashtag data

### Testing Framework

- **Property-based testing library**: `proptest` (Rust's standard PBT library)
- **Unit testing**: Rust's built-in `#[test]` framework
- **Test database**: SQLite in-memory database for fast, isolated tests
- **Minimum iterations**: 100 iterations per property test

Each property-based test will be tagged with a comment referencing the design document:
```rust
// Feature: hashtag-backfill-migration, Property 1: Migration completeness and accuracy
#[test]
fn prop_migration_completeness() { ... }
```

## Implementation Details

### Migration Algorithm

```
1. Parse command-line arguments
2. Connect to database
3. If not --yes flag, display confirmation prompt
4. Query all posts: SELECT id, content FROM posts
5. Initialize statistics tracker
6. For each post:
   a. Extract hashtags from content using extract_hashtags()
   b. If hashtags found:
      - Call hashtag_repo.store_hashtags(post_id, hashtags)
      - Update statistics
   c. If error occurs:
      - Log error with post ID
      - Continue to next post
7. Display migration summary with statistics
8. Exit with appropriate status code
```

### Transaction Strategy

To balance safety and performance:
- Use a single transaction for the entire migration (rollback on fatal error)
- Use `INSERT OR IGNORE` for idempotent hashtag and association creation
- This ensures atomicity while allowing the migration to be safely re-run

### Performance Considerations

- **Batch processing**: Process posts in batches of 100 to reduce memory usage
- **Progress reporting**: Display progress every 100 posts for long-running migrations
- **Index usage**: Leverage existing database indexes on `hashtags.name` and `post_hashtags.post_id`
- **Expected performance**: ~1000 posts/second on typical hardware

### Dry-Run Mode

When `--dry-run` flag is provided:
- Open database in read-only mode
- Extract hashtags and calculate statistics
- Display what would be done without making changes
- Useful for previewing migration impact

## Configuration

### Command-Line Interface

```bash
# Basic usage with default database
fido-migrate

# Specify custom database path
fido-migrate --database /path/to/fido.db

# Dry run to preview changes
fido-migrate --dry-run

# Skip confirmation prompt
fido-migrate --yes

# Combined flags
fido-migrate --database ./custom.db --dry-run
```

### Exit Codes

- `0`: Migration completed successfully
- `1`: Fatal error (database connection failed, invalid schema)
- `2`: Partial failure (some posts failed to process, but migration completed)

## Dependencies

### Cargo.toml for fido-migrate

```toml
[package]
name = "fido-migrate"
version = "0.1.0"
edition = "2021"

[dependencies]
fido-server = { path = "../fido-server" }
fido-types = { path = "../fido-types" }
anyhow = "1.0"
clap = { version = "4.0", features = ["derive"] }
uuid = { version = "1.0", features = ["v4"] }

[dev-dependencies]
proptest = "1.0"
```

## Future Enhancements

Potential improvements for future iterations:
- Progress bar for visual feedback during long migrations
- Parallel processing for large databases
- Export migration report to JSON file
- Rollback capability with backup creation
- Verification mode to check migration correctness without re-running
