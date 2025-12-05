# Requirements Document

## Introduction

This feature implements a one-time data migration script to backfill hashtag data for existing posts in the Fido database. Currently, hashtags are extracted from post content for display purposes only, but are not persisted to the database tables. This prevents hashtag filtering from working on older posts. The migration will read all existing posts, extract hashtags from their content, and populate the hashtag-related database tables.

## Glossary

- **Migration Script**: A one-time executable program that updates existing database records
- **Hashtag**: A word or phrase prefixed with '#' symbol (e.g., #rust, #coding)
- **Post Content**: The markdown text body of a post that may contain hashtags
- **Hashtag Extraction**: The process of parsing post content to identify hashtag patterns
- **Database Tables**: The hashtags and post_hashtags tables that store hashtag relationships

## Requirements

### Requirement 1

**User Story:** As a system administrator, I want to run a migration script to backfill hashtags for existing posts, so that hashtag filtering works consistently across all posts regardless of creation date.

#### Acceptance Criteria

1. WHEN the migration script is executed THEN the system SHALL read all posts from the posts table
2. WHEN processing each post THEN the system SHALL extract all hashtags from the post content using the same extraction logic as the UI display
3. WHEN a hashtag is extracted THEN the system SHALL insert it into the hashtags table if it does not already exist
4. WHEN a hashtag is associated with a post THEN the system SHALL create a record in the post_hashtags table linking the post_id and hashtag_id
5. WHEN the migration completes THEN the system SHALL report the number of posts processed and hashtags created

### Requirement 2

**User Story:** As a developer, I want the migration script to handle edge cases and errors gracefully, so that the migration can complete successfully without corrupting data.

#### Acceptance Criteria

1. WHEN duplicate hashtag-post associations already exist THEN the system SHALL skip creating duplicate records
2. WHEN a database error occurs during migration THEN the system SHALL log the error and continue processing remaining posts
3. WHEN post content contains malformed or invalid hashtags THEN the system SHALL skip invalid hashtags and continue processing
4. WHEN the migration script is run multiple times THEN the system SHALL produce idempotent results without creating duplicate data
5. WHEN processing completes THEN the system SHALL commit all changes as a single transaction or rollback on failure

### Requirement 3

**User Story:** As a developer, I want the migration script to be easy to run and integrate with the existing codebase, so that it can be executed as part of deployment or maintenance procedures.

#### Acceptance Criteria

1. WHEN the migration script is invoked THEN the system SHALL accept a database path as a command-line argument
2. WHEN no database path is provided THEN the system SHALL use the default database location from the application configuration
3. WHEN the script starts THEN the system SHALL display a confirmation prompt before making any database changes
4. WHEN the migration is running THEN the system SHALL display progress information for long-running operations
5. WHEN the migration completes THEN the system SHALL display a summary of changes made
