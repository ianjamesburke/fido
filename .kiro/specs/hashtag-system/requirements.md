# Requirements Document

## Introduction

This specification defines a comprehensive hashtag system for the Fido terminal application. The system enables users to organize, discover, and filter content using hashtags, providing a powerful way to follow topics of interest and track engagement. The implementation includes backend infrastructure for hashtag storage and tracking, as well as a simplified UI for filtering and managing hashtags.

## Glossary

- **Hashtag**: A word or phrase prefixed with '#' used to categorize and discover content (e.g., #rust, #web_dev, #rust2024)
- **Hashtag Following**: A user's subscription to a hashtag to see posts containing that hashtag
- **Hashtag Activity**: Metrics tracking user interactions with hashtags (posts, votes, views)
- **Filter Modal**: A modal interface for selecting content filters (hashtags, users, or all posts)
- **Post Extraction**: The process of parsing hashtags from post content during creation
- **Sort Order**: The arrangement of posts (Newest, Top, Hot) applied to filtered feeds

## Requirements

### Requirement 1: Hashtag Data Model and Storage

**User Story:** As a developer, I want a robust data model for hashtags, so that the system can efficiently store and query hashtag-related data.

#### Acceptance Criteria

1. THE System SHALL store hashtags in a dedicated table with unique names and creation timestamps
2. THE System SHALL maintain a many-to-many relationship between posts and hashtags
3. THE System SHALL track which hashtags each user follows with timestamps
4. THE System SHALL record user activity metrics for each hashtag including interaction count and last interaction time
5. THE System SHALL use foreign key constraints to maintain referential integrity between tables
6. THE System SHALL generate unique identifiers for all hashtag records

### Requirement 2: Hashtag Parsing and Extraction

**User Story:** As a user, I want hashtags to be automatically extracted from my posts, so that I don't need to manually tag my content.

#### Acceptance Criteria

1. WHEN a user creates a post, THE System SHALL extract all hashtags from the post content using the pattern #\w+
2. THE System SHALL support hashtags containing letters, numbers, and underscores
3. WHEN a hashtag does not exist in the database, THE System SHALL create a new hashtag entry automatically
4. WHEN a post contains multiple hashtags, THE System SHALL link all extracted hashtags to the post
5. THE System SHALL extract hashtags case-insensitively but preserve the original case in storage
6. THE System SHALL support hashtags with minimum length of 2 characters

### Requirement 3: Hashtag Following Management

**User Story:** As a user, I want to follow hashtags that interest me, so that I can easily filter content by topics I care about.

#### Acceptance Criteria

1. WHEN a user follows a hashtag, THE System SHALL record the follow relationship with a timestamp
2. WHEN a user unfollows a hashtag, THE System SHALL remove the follow relationship
3. WHEN a user requests their followed hashtags, THE System SHALL return a list of all hashtags they follow
4. THE System SHALL prevent duplicate follow relationships for the same user and hashtag
5. THE System SHALL allow users to follow any hashtag that exists in the system

### Requirement 4: Hashtag-Based Post Filtering

**User Story:** As a user, I want to filter posts by hashtag, so that I can view content related to specific topics.

#### Acceptance Criteria

1. WHEN a user requests posts filtered by hashtag, THE System SHALL return only posts containing that hashtag
2. WHEN filtering by hashtag, THE System SHALL apply the user's selected sort order (Newest, Top, Hot)
3. WHEN a hashtag has no posts, THE System SHALL return an empty list without errors
4. THE System SHALL support filtering by hashtag name (case-insensitive matching)
5. THE System SHALL include all post metadata (author, votes, timestamps) in filtered results

### Requirement 5: Filter Modal UI

**User Story:** As a user, I want a simple modal interface to select content filters, so that I can quickly switch between viewing all posts, hashtag-filtered posts, or user-filtered posts.

#### Acceptance Criteria

1. WHEN the user presses 'F' on the Posts page, THE TUI SHALL display a filter modal with three tabs: Hashtags, Users, and All
2. WHEN the Hashtags tab is active, THE TUI SHALL display a list of followed hashtags
3. WHEN the Users tab is active, THE TUI SHALL display a list of bookmarked users
4. WHEN the All tab is active, THE TUI SHALL provide an option to return to the global feed
5. WHEN the user selects a filter, THE TUI SHALL close the modal and apply the selected filter
6. WHEN the user presses Escape in the filter modal, THE TUI SHALL close the modal without changing the current filter

### Requirement 6: Filter Display and Navigation

**User Story:** As a user, I want to see which filter is currently active, so that I understand what content I'm viewing.

#### Acceptance Criteria

1. WHEN viewing the Posts page, THE TUI SHALL display the current filter at the top (e.g., "[All Posts]", "[#rust]", "[@username]")
2. WHEN a hashtag filter is active, THE TUI SHALL display the hashtag name with the # prefix
3. WHEN a user filter is active, THE TUI SHALL display the username with the @ prefix
4. WHEN no filter is active, THE TUI SHALL display "[All Posts]"
5. THE TUI SHALL update the filter display immediately when the user changes filters

### Requirement 7: Filter Modal Navigation

**User Story:** As a user, I want to navigate the filter modal with keyboard shortcuts, so that I can quickly select filters without using a mouse.

#### Acceptance Criteria

1. WHEN the filter modal is open, THE TUI SHALL allow navigation between tabs using Tab or arrow keys
2. WHEN a tab is active, THE TUI SHALL allow navigation within the list using j/k or arrow keys
3. WHEN the user presses Enter on a filter item, THE TUI SHALL apply that filter and close the modal
4. WHEN the user presses 'X' on a followed hashtag, THE TUI SHALL unfollow that hashtag
5. WHEN the user presses 'X' on a bookmarked user, THE TUI SHALL remove that bookmark
6. THE TUI SHALL provide visual feedback for the currently selected tab and item

### Requirement 8: Hashtag Search and Discovery

**User Story:** As a user, I want to search for hashtags, so that I can discover and follow new topics.

#### Acceptance Criteria

1. WHEN the user selects "Search/Add New" in the Hashtags tab, THE TUI SHALL display a search input field
2. WHEN the user types in the search field, THE TUI SHALL query available hashtags matching the input
3. WHEN search results are displayed, THE TUI SHALL show hashtag names and post counts
4. WHEN the user selects a hashtag from search results, THE TUI SHALL follow that hashtag and add it to the followed list
5. THE System SHALL search hashtags by partial name match (case-insensitive)

### Requirement 9: Sort Order Integration

**User Story:** As a user, I want to cycle through sort orders while viewing filtered content, so that I can see posts arranged in different ways.

#### Acceptance Criteria

1. WHEN the user presses 'S' on the Posts page, THE TUI SHALL cycle through sort orders: Newest → Top → Hot → Newest
2. WHEN the sort order changes, THE TUI SHALL display the current sort order in the filter bar (e.g., "[#rust] (Sorted by: Top)")
3. WHEN the user changes filters, THE TUI SHALL preserve the current sort order
4. THE System SHALL persist sort preferences per filter context (global, hashtag, user)
5. THE System SHALL store sort preferences in local configuration

### Requirement 10: Hashtag Activity Tracking

**User Story:** As a user, I want the system to track my hashtag activity, so that I can see which topics I engage with most.

#### Acceptance Criteria

1. WHEN a user creates a post with hashtags, THE System SHALL increment the interaction count for those hashtags
2. WHEN a user votes on a post with hashtags, THE System SHALL increment the interaction count for those hashtags
3. WHEN a user views posts filtered by a hashtag, THE System SHALL update the last interaction timestamp
4. THE System SHALL maintain separate activity metrics for each user-hashtag pair
5. THE System SHALL use activity metrics to display "Most Active Hashtags" on the profile page

