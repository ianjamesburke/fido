# Implementation Plan

- [x] 1. Set up migration utility crate structure



  - Create `fido-migrate` directory in workspace
  - Create `Cargo.toml` with dependencies on `fido-server`, `fido-types`, `clap`, `anyhow`
  - Create `src/main.rs` entry point
  - Add `fido-migrate` to workspace `Cargo.toml`
  - _Requirements: 3.1, 3.2_



- [x] 2. Implement command-line argument parsing

  - Define `Args` struct with `clap` derive macros
  - Add `database` field with default value `./fido.db`
  - Add `dry_run` boolean flag
  - Add `yes` boolean flag for skipping confirmation

  - Parse and validate arguments in main function
  - _Requirements: 3.1, 3.2_

- [x] 3. Implement migration statistics tracking

  - Create `MigrationStats` struct with counters
  - Implement methods to update statistics
  - Implement `display_stats` function for formatted output
  - _Requirements: 1.5_

- [x] 4. Implement core migration logic


  - [x] 4.1 Create database connection function


    - Connect to SQLite database using provided path
    - Validate database schema exists
    - Return appropriate errors for connection failures
    - _Requirements: 3.1, 3.2_

  - [x] 4.2 Implement post querying


    - Query all posts from database: `SELECT id, content FROM posts`
    - Handle empty result set gracefully
    - _Requirements: 1.1_

  - [x] 4.3 Implement hashtag extraction and storage per post



    - For each post, call `extract_hashtags` on content
    - Use `HashtagRepository::store_hashtags` to save hashtags
    - Track statistics for each post processed
    - Handle errors gracefully and continue processing
    - _Requirements: 1.2, 1.3, 1.4_

  - [x] 4.4 Write property test for extraction consistency


    - **Property 2: Extraction consistency**
    - **Validates: Requirements 1.2**

  - [x] 4.5 Write property test for association correctness


    - **Property 3: Association correctness**
    - **Validates: Requirements 1.4**

- [x] 5. Implement transaction and error handling


  - Wrap migration in single database transaction
  - Use `INSERT OR IGNORE` for idempotent operations
  - Collect errors during processing
  - Display errors in final summary
  - _Requirements: 2.1, 2.2, 2.5_

- [x] 5.1 Write property test for idempotency


  - **Property 4: Migration idempotency**
  - **Validates: Requirements 1.3, 2.1, 2.4**

- [x] 6. Implement user interaction features


  - Add confirmation prompt before migration (unless `--yes` flag)
  - Display progress information during migration
  - Display final summary with statistics
  - _Requirements: 3.3, 3.4, 3.5_

- [x] 7. Implement dry-run mode

  - Check for `--dry-run` flag
  - Open database in read-only mode
  - Extract and count hashtags without storing
  - Display what would be done
  - _Requirements: 2.4_

- [x] 8. Write property test for completeness


  - **Property 1: Migration completeness and accuracy**
  - **Validates: Requirements 1.1, 1.5**

- [x] 9. Write property test for robustness


  - **Property 5: Robustness to edge cases**
  - **Validates: Requirements 2.3**

- [x] 10. Write unit tests for edge cases

  - Test with empty database
  - Test with posts containing no hashtags
  - Test with posts containing special characters
  - Test with very long post content
  - Test command-line argument parsing variations
  - _Requirements: 2.3_

- [x] 11. Add documentation and usage instructions


  - Add doc comments to main functions
  - Create README with usage examples
  - Document exit codes
  - Add examples for common use cases
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 12. Checkpoint - Ensure all tests pass



  - Ensure all tests pass, ask the user if questions arise.
