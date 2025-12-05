# Requirements Document

## Introduction

This feature addresses a critical bug where hashtag filtering in the Filter Posts window only shows posts created after toggling the hashtag filter, not historical posts that contain the hashtag. The root cause is that the database schema hasn't been updated to include the hashtag tables, preventing the backfill migration from running and populating hashtag associations for existing posts.

## Glossary

- **Hashtag Filter**: A UI feature that allows users to filter the global feed to show only posts containing specific hashtags
- **Backfill Migration**: A one-time data migration that extracts hashtags from existing post content and populates the hashtag relationship tables
- **Database Schema**: The structure of database tables, columns, and relationships defined in the application
- **Post-Hashtag Association**: A database record linking a post to a hashtag it contains
- **Filter Posts Window**: The UI modal where users can toggle friends and hashtags to filter the feed

## Requirements

### Requirement 1

**User Story:** As a user, I want hashtag filtering to show all posts containing the selected hashtag, so that I can see both historical and new posts with that hashtag.

#### Acceptance Criteria

1. WHEN the database is initialized THEN the system SHALL create all required hashtag tables (hashtags, post_hashtags, user_hashtag_follows, user_hashtag_activity)
2. WHEN the backfill migration is executed THEN the system SHALL populate hashtag associations for all existing posts
3. WHEN a user toggles a hashtag filter and presses Enter THEN the system SHALL display all posts containing that hashtag regardless of creation date
4. WHEN the feed is filtered by hashtag THEN the system SHALL query the post_hashtags table to find matching posts
5. WHEN a new post is created with hashtags THEN the system SHALL store hashtag associations in the post_hashtags table

### Requirement 2

**User Story:** As a developer, I want to verify the database schema is correct and the migration has run successfully, so that I can confirm the bug is fixed.

#### Acceptance Criteria

1. WHEN the database file is inspected THEN the system SHALL contain the hashtags table with appropriate columns
2. WHEN the database file is inspected THEN the system SHALL contain the post_hashtags table with post_id and hashtag_id columns
3. WHEN the backfill migration completes THEN the system SHALL report the number of posts processed and hashtag associations created
4. WHEN querying the post_hashtags table THEN the system SHALL return associations for posts created before the migration
5. WHEN the application starts THEN the system SHALL use the current schema definition that includes all hashtag tables

### Requirement 3

**User Story:** As a user, I want the hashtag filter to work consistently with the friends filter, so that I have a predictable filtering experience.

#### Acceptance Criteria

1. WHEN both friend and hashtag filters are active THEN the system SHALL show posts that match both criteria
2. WHEN multiple hashtags are selected THEN the system SHALL show posts containing any of the selected hashtags
3. WHEN no filters are active THEN the system SHALL show all posts in the global feed
4. WHEN filters are applied THEN the system SHALL maintain the selected sort order (Newest, Popular, Controversial)
5. WHEN the filter modal is closed THEN the system SHALL persist the filter selections until changed by the user
