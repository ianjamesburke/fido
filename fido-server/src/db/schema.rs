/// SQL schema for the Fido database
/// Creates all tables with proper constraints, foreign keys, and indexes
pub const SCHEMA: &str = r#"
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    bio TEXT,
    join_date TEXT NOT NULL,
    is_test_user INTEGER NOT NULL DEFAULT 0
);

-- Posts table
CREATE TABLE IF NOT EXISTS posts (
    id TEXT PRIMARY KEY,
    author_id TEXT NOT NULL,
    content TEXT NOT NULL CHECK(length(content) <= 280),
    created_at TEXT NOT NULL,
    upvotes INTEGER NOT NULL DEFAULT 0,
    downvotes INTEGER NOT NULL DEFAULT 0,
    parent_post_id TEXT,
    reply_to_user_id TEXT,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (reply_to_user_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Create index on created_at for efficient post sorting
CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at DESC);

-- Create index on parent_post_id for efficient reply lookups
CREATE INDEX IF NOT EXISTS idx_posts_parent_post_id ON posts(parent_post_id);

-- Hashtags table (unique hashtag names)
CREATE TABLE IF NOT EXISTS hashtags (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    created_at INTEGER NOT NULL
);

-- Post-hashtag junction table
CREATE TABLE IF NOT EXISTS post_hashtags (
    post_id TEXT NOT NULL,
    hashtag_id TEXT NOT NULL,
    PRIMARY KEY (post_id, hashtag_id),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (hashtag_id) REFERENCES hashtags(id) ON DELETE CASCADE
);

-- User hashtag follows
CREATE TABLE IF NOT EXISTS user_hashtag_follows (
    user_id TEXT NOT NULL,
    hashtag_id TEXT NOT NULL,
    followed_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, hashtag_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (hashtag_id) REFERENCES hashtags(id) ON DELETE CASCADE
);

-- User hashtag activity tracking
CREATE TABLE IF NOT EXISTS user_hashtag_activity (
    user_id TEXT NOT NULL,
    hashtag_id TEXT NOT NULL,
    interaction_count INTEGER DEFAULT 0,
    last_interaction INTEGER,
    PRIMARY KEY (user_id, hashtag_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (hashtag_id) REFERENCES hashtags(id) ON DELETE CASCADE
);

-- Indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_hashtags_name ON hashtags(name);
CREATE INDEX IF NOT EXISTS idx_post_hashtags_post ON post_hashtags(post_id);
CREATE INDEX IF NOT EXISTS idx_post_hashtags_hashtag ON post_hashtags(hashtag_id);
CREATE INDEX IF NOT EXISTS idx_user_hashtag_follows_user ON user_hashtag_follows(user_id);
CREATE INDEX IF NOT EXISTS idx_user_hashtag_activity_user ON user_hashtag_activity(user_id);

-- Votes table
CREATE TABLE IF NOT EXISTS votes (
    user_id TEXT NOT NULL,
    post_id TEXT NOT NULL,
    direction TEXT NOT NULL CHECK(direction IN ('up', 'down')),
    created_at TEXT NOT NULL,
    PRIMARY KEY (user_id, post_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
);

-- Create index on user_id for efficient vote lookups
CREATE INDEX IF NOT EXISTS idx_votes_user_id ON votes(user_id);
CREATE INDEX IF NOT EXISTS idx_votes_post_id ON votes(post_id);

-- Follows table (one-way relationships)
CREATE TABLE IF NOT EXISTS follows (
    follower_id TEXT NOT NULL,
    following_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (follower_id, following_id),
    FOREIGN KEY (follower_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (following_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Indexes for efficient follow lookups
CREATE INDEX IF NOT EXISTS idx_follows_follower ON follows(follower_id);
CREATE INDEX IF NOT EXISTS idx_follows_following ON follows(following_id);

-- Legacy friendships table (for backward compatibility during migration)
CREATE TABLE IF NOT EXISTS friendships (
    user_id TEXT NOT NULL,
    friend_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, friend_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (friend_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Indexes for efficient friend lookups
CREATE INDEX IF NOT EXISTS idx_friendships_user ON friendships(user_id);
CREATE INDEX IF NOT EXISTS idx_friendships_friend ON friendships(friend_id);

-- Direct messages table
CREATE TABLE IF NOT EXISTS direct_messages (
    id TEXT PRIMARY KEY,
    from_user_id TEXT NOT NULL,
    to_user_id TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    is_read INTEGER NOT NULL DEFAULT 0,
    deleted_by_from_user INTEGER NOT NULL DEFAULT 0,
    deleted_by_to_user INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (from_user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (to_user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create indexes for efficient DM queries
CREATE INDEX IF NOT EXISTS idx_dms_from_user ON direct_messages(from_user_id);
CREATE INDEX IF NOT EXISTS idx_dms_to_user ON direct_messages(to_user_id);
CREATE INDEX IF NOT EXISTS idx_dms_created_at ON direct_messages(created_at DESC);

-- User configurations table
CREATE TABLE IF NOT EXISTS user_configs (
    user_id TEXT PRIMARY KEY,
    color_scheme TEXT NOT NULL DEFAULT 'Default',
    sort_order TEXT NOT NULL DEFAULT 'Newest',
    max_posts_display INTEGER NOT NULL DEFAULT 25,
    emoji_enabled INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
"#;

/// Test data for development and testing
/// Includes comprehensive test data for all features:
/// - 3 test users (alice, bob, charlie)
/// - Multiple posts with various hashtags
/// - Votes demonstrating upvote/downvote functionality
/// - Direct message conversations between users
/// - User configurations with different preferences
pub const TEST_DATA: &str = r#"
-- ============================================================================
-- TEST USERS
-- ============================================================================
-- Insert test users with diverse profiles
INSERT OR IGNORE INTO users (id, username, bio, join_date, is_test_user) VALUES
    ('550e8400-e29b-41d4-a716-446655440001', 'alice', 'Rust enthusiast and terminal lover 🦀', '2024-01-01T00:00:00Z', 1),
    ('550e8400-e29b-41d4-a716-446655440002', 'bob', 'Terminal UI designer and developer 🎨', '2024-01-02T00:00:00Z', 1),
    ('550e8400-e29b-41d4-a716-446655440003', 'charlie', 'SQLite advocate and database expert 💾', '2024-01-03T00:00:00Z', 1),
    ('550e8400-e29b-41d4-a716-446655440004', 'diana', 'Open source maintainer | Coffee addict ☕', '2024-01-04T00:00:00Z', 1),
    ('550e8400-e29b-41d4-a716-446655440005', 'eve', 'DevOps engineer | Automation enthusiast 🤖', '2024-01-05T00:00:00Z', 1),
    ('550e8400-e29b-41d4-a716-446655440006', 'frank', 'Systems programmer | Low-level wizard ⚡', '2024-01-06T00:00:00Z', 1),
    ('550e8400-e29b-41d4-a716-446655440007', 'grace', 'Security researcher | Bug bounty hunter 🔒', '2024-01-07T00:00:00Z', 1),
    ('550e8400-e29b-41d4-a716-446655440008', 'hank', 'Performance optimization nerd | Benchmarking 📊', '2024-01-08T00:00:00Z', 1);

-- ============================================================================
-- USER CONFIGURATIONS
-- ============================================================================
-- Insert default configs for test users with different preferences
INSERT OR IGNORE INTO user_configs (user_id, color_scheme, sort_order, max_posts_display, emoji_enabled) VALUES
    ('550e8400-e29b-41d4-a716-446655440001', 'Default', 'Newest', 25, 1),
    ('550e8400-e29b-41d4-a716-446655440002', 'Dark', 'Popular', 50, 1),
    ('550e8400-e29b-41d4-a716-446655440003', 'Solarized', 'Newest', 30, 1),
    ('550e8400-e29b-41d4-a716-446655440004', 'Default', 'Popular', 40, 1),
    ('550e8400-e29b-41d4-a716-446655440005', 'Dark', 'Newest', 35, 1),
    ('550e8400-e29b-41d4-a716-446655440006', 'Solarized', 'Popular', 20, 1),
    ('550e8400-e29b-41d4-a716-446655440007', 'Default', 'Newest', 50, 1),
    ('550e8400-e29b-41d4-a716-446655440008', 'Dark', 'Popular', 25, 1);

-- ============================================================================
-- SAMPLE POSTS
-- ============================================================================
-- Insert diverse posts with various hashtags, emojis, and content
INSERT OR IGNORE INTO posts (id, author_id, content, created_at, upvotes, downvotes) VALUES
    -- Alice's posts
    ('650e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440001', 'Just shipped a new #rust feature! 🚀 The performance improvements are incredible.', '2024-01-10T10:00:00Z', 15, 1),
    ('650e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440001', 'Learning about #async #rust patterns. The borrow checker is my friend! 💪', '2024-01-09T14:30:00Z', 8, 0),
    ('650e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440001', 'Hot take: #terminal apps are the future of developer tools. No bloat, just speed. ⚡', '2024-01-08T09:15:00Z', 23, 3),
    ('650e8400-e29b-41d4-a716-446655440004', '550e8400-e29b-41d4-a716-446655440001', 'Anyone else obsessed with #ratatui? Building TUIs has never been easier! 🎯', '2024-01-07T16:45:00Z', 12, 0),
    
    -- Bob's posts
    ('650e8400-e29b-41d4-a716-446655440005', '550e8400-e29b-41d4-a716-446655440002', 'Working on #terminal #ui design. Any tips for better color schemes? 🎨', '2024-01-10T08:00:00Z', 18, 2),
    ('650e8400-e29b-41d4-a716-446655440006', '550e8400-e29b-41d4-a716-446655440002', 'Just discovered #crossterm for terminal manipulation. Game changer! 🔥', '2024-01-09T11:20:00Z', 10, 0),
    ('650e8400-e29b-41d4-a716-446655440007', '550e8400-e29b-41d4-a716-446655440002', 'Keyboard-driven interfaces are so much faster than mouse-based UIs. #productivity #terminal', '2024-01-08T13:00:00Z', 14, 1),
    ('650e8400-e29b-41d4-a716-446655440008', '550e8400-e29b-41d4-a716-446655440002', 'Pro tip: Use vim keybindings everywhere. Your fingers will thank you! ⌨️ #vim #productivity', '2024-01-07T10:30:00Z', 20, 4),
    
    -- Charlie's posts
    ('650e8400-e29b-41d4-a716-446655440009', '550e8400-e29b-41d4-a716-446655440003', 'Love the simplicity of #sqlite for MVPs. Perfect for rapid prototyping! 💡', '2024-01-10T06:00:00Z', 16, 0),
    ('650e8400-e29b-41d4-a716-446655440010', '550e8400-e29b-41d4-a716-446655440003', '#sqlite is underrated. It powers more apps than you think! 📱', '2024-01-09T15:45:00Z', 11, 1),
    ('650e8400-e29b-41d4-a716-446655440011', '550e8400-e29b-41d4-a716-446655440003', 'Database indexing 101: Always index your foreign keys! #database #performance', '2024-01-08T12:00:00Z', 9, 0),
    ('650e8400-e29b-41d4-a716-446655440012', '550e8400-e29b-41d4-a716-446655440003', 'Controversial opinion: Not every app needs PostgreSQL. #sqlite #database', '2024-01-07T14:20:00Z', 25, 8),
    
    -- More collaborative posts
    ('650e8400-e29b-41d4-a716-446655440013', '550e8400-e29b-41d4-a716-446655440001', 'Shoutout to @bob for the amazing #ui work on Fido! 🙌', '2024-01-06T17:00:00Z', 19, 0),
    ('650e8400-e29b-41d4-a716-446655440014', '550e8400-e29b-41d4-a716-446655440002', 'Thanks @alice! Couldn''t have done it without your #rust expertise. 🤝', '2024-01-06T17:15:00Z', 15, 0),
    ('650e8400-e29b-41d4-a716-446655440015', '550e8400-e29b-41d4-a716-446655440003', 'This team is crushing it! #fido #rust #terminal #collaboration', '2024-01-06T17:30:00Z', 22, 0),
    
    -- Diana's posts (open source maintainer)
    ('650e8400-e29b-41d4-a716-446655440016', '550e8400-e29b-41d4-a716-446655440004', 'Just merged 50 PRs today! Open source never sleeps 😅 #opensource #maintainer', '2024-01-10T12:00:00Z', 28, 2),
    ('650e8400-e29b-41d4-a716-446655440017', '550e8400-e29b-41d4-a716-446655440004', 'Coffee count: 7 ☕ Bug fixes: 12 🐛 Good day! #coding #coffee', '2024-01-09T18:00:00Z', 21, 0),
    ('650e8400-e29b-41d4-a716-446655440018', '550e8400-e29b-41d4-a716-446655440004', 'Remember: Good documentation is love for your future self ❤️ #documentation #opensource', '2024-01-08T11:00:00Z', 35, 1),
    
    -- Eve's posts (DevOps engineer)
    ('650e8400-e29b-41d4-a716-446655440019', '550e8400-e29b-41d4-a716-446655440005', 'Automated all the things! CI/CD pipeline running smooth 🚀 #devops #automation', '2024-01-10T09:30:00Z', 24, 0),
    ('650e8400-e29b-41d4-a716-446655440020', '550e8400-e29b-41d4-a716-446655440005', 'Infrastructure as code is the only way. Fight me. 🤖 #terraform #kubernetes', '2024-01-09T13:00:00Z', 31, 5),
    ('650e8400-e29b-41d4-a716-446655440021', '550e8400-e29b-41d4-a716-446655440005', 'Monitoring dashboards looking beautiful today 📊 #observability #metrics', '2024-01-08T15:30:00Z', 18, 0),
    
    -- Frank's posts (systems programmer)
    ('650e8400-e29b-41d4-a716-446655440022', '550e8400-e29b-41d4-a716-446655440006', 'Writing assembly for fun on a Friday night ⚡ #lowlevel #systems', '2024-01-10T20:00:00Z', 16, 3),
    ('650e8400-e29b-41d4-a716-446655440023', '550e8400-e29b-41d4-a716-446655440006', 'Memory safety without garbage collection? That''s why I love #rust 🦀', '2024-01-09T10:00:00Z', 27, 1),
    ('650e8400-e29b-41d4-a716-446655440024', '550e8400-e29b-41d4-a716-446655440006', 'Kernel hacking is meditation for programmers 🧘 #linux #kernel', '2024-01-08T08:00:00Z', 22, 2),
    
    -- Grace's posts (security researcher)
    ('650e8400-e29b-41d4-a716-446655440025', '550e8400-e29b-41d4-a716-446655440007', 'Found another critical CVE today 🔒 Responsible disclosure in progress #security #bugbounty', '2024-01-10T14:00:00Z', 42, 0),
    ('650e8400-e29b-41d4-a716-446655440026', '550e8400-e29b-41d4-a716-446655440007', 'PSA: Always sanitize your inputs! SQL injection is still a thing in 2024 😱 #security #appsec', '2024-01-09T16:30:00Z', 38, 1),
    ('650e8400-e29b-41d4-a716-446655440027', '550e8400-e29b-41d4-a716-446655440007', 'Fuzzing is my favorite pastime 🎯 #security #fuzzing #testing', '2024-01-08T19:00:00Z', 19, 0),
    
    -- Hank's posts (performance optimization)
    ('650e8400-e29b-41d4-a716-446655440028', '550e8400-e29b-41d4-a716-446655440008', 'Shaved 200ms off the hot path! Benchmarks don''t lie 📊 #performance #optimization', '2024-01-10T11:30:00Z', 33, 0),
    ('650e8400-e29b-41d4-a716-446655440029', '550e8400-e29b-41d4-a716-446655440008', 'Premature optimization is the root of all evil, but profiling is divine ⚡ #performance', '2024-01-09T12:00:00Z', 29, 2),
    ('650e8400-e29b-41d4-a716-446655440030', '550e8400-e29b-41d4-a716-446655440008', 'Cache invalidation: the hardest problem in computer science 🤔 #caching #performance', '2024-01-08T14:00:00Z', 26, 1);

-- ============================================================================
-- HASHTAGS
-- ============================================================================
-- Note: Hashtags are automatically extracted and stored when posts are created
-- via the HashtagRepository. No manual insertion needed for test data.

-- ============================================================================
-- VOTES
-- ============================================================================
-- Insert votes demonstrating various voting patterns
INSERT OR IGNORE INTO votes (user_id, post_id, direction, created_at) VALUES
    -- Votes on Alice's posts
    ('550e8400-e29b-41d4-a716-446655440002', '650e8400-e29b-41d4-a716-446655440001', 'up', '2024-01-10T10:05:00Z'),
    ('550e8400-e29b-41d4-a716-446655440003', '650e8400-e29b-41d4-a716-446655440001', 'up', '2024-01-10T10:10:00Z'),
    ('550e8400-e29b-41d4-a716-446655440002', '650e8400-e29b-41d4-a716-446655440002', 'up', '2024-01-09T15:00:00Z'),
    ('550e8400-e29b-41d4-a716-446655440003', '650e8400-e29b-41d4-a716-446655440003', 'up', '2024-01-08T10:00:00Z'),
    ('550e8400-e29b-41d4-a716-446655440002', '650e8400-e29b-41d4-a716-446655440003', 'down', '2024-01-08T10:30:00Z'),
    
    -- Votes on Bob's posts
    ('550e8400-e29b-41d4-a716-446655440001', '650e8400-e29b-41d4-a716-446655440005', 'up', '2024-01-10T08:15:00Z'),
    ('550e8400-e29b-41d4-a716-446655440003', '650e8400-e29b-41d4-a716-446655440005', 'up', '2024-01-10T08:30:00Z'),
    ('550e8400-e29b-41d4-a716-446655440001', '650e8400-e29b-41d4-a716-446655440006', 'up', '2024-01-09T12:00:00Z'),
    ('550e8400-e29b-41d4-a716-446655440003', '650e8400-e29b-41d4-a716-446655440007', 'up', '2024-01-08T14:00:00Z'),
    ('550e8400-e29b-41d4-a716-446655440001', '650e8400-e29b-41d4-a716-446655440008', 'up', '2024-01-07T11:00:00Z'),
    ('550e8400-e29b-41d4-a716-446655440003', '650e8400-e29b-41d4-a716-446655440008', 'down', '2024-01-07T11:30:00Z'),
    
    -- Votes on Charlie's posts
    ('550e8400-e29b-41d4-a716-446655440001', '650e8400-e29b-41d4-a716-446655440009', 'up', '2024-01-10T07:00:00Z'),
    ('550e8400-e29b-41d4-a716-446655440002', '650e8400-e29b-41d4-a716-446655440009', 'up', '2024-01-10T07:15:00Z'),
    ('550e8400-e29b-41d4-a716-446655440001', '650e8400-e29b-41d4-a716-446655440010', 'up', '2024-01-09T16:00:00Z'),
    ('550e8400-e29b-41d4-a716-446655440002', '650e8400-e29b-41d4-a716-446655440011', 'up', '2024-01-08T13:00:00Z'),
    ('550e8400-e29b-41d4-a716-446655440001', '650e8400-e29b-41d4-a716-446655440012', 'up', '2024-01-07T15:00:00Z'),
    ('550e8400-e29b-41d4-a716-446655440002', '650e8400-e29b-41d4-a716-446655440012', 'down', '2024-01-07T15:30:00Z'),
    
    -- Votes on collaborative posts
    ('550e8400-e29b-41d4-a716-446655440002', '650e8400-e29b-41d4-a716-446655440013', 'up', '2024-01-06T17:05:00Z'),
    ('550e8400-e29b-41d4-a716-446655440003', '650e8400-e29b-41d4-a716-446655440013', 'up', '2024-01-06T17:10:00Z'),
    ('550e8400-e29b-41d4-a716-446655440001', '650e8400-e29b-41d4-a716-446655440014', 'up', '2024-01-06T17:20:00Z'),
    ('550e8400-e29b-41d4-a716-446655440003', '650e8400-e29b-41d4-a716-446655440014', 'up', '2024-01-06T17:25:00Z'),
    ('550e8400-e29b-41d4-a716-446655440001', '650e8400-e29b-41d4-a716-446655440015', 'up', '2024-01-06T17:35:00Z'),
    ('550e8400-e29b-41d4-a716-446655440002', '650e8400-e29b-41d4-a716-446655440015', 'up', '2024-01-06T17:40:00Z');

-- ============================================================================
-- DIRECT MESSAGES
-- ============================================================================
-- Insert sample direct messages demonstrating conversations between users
INSERT OR IGNORE INTO direct_messages (id, from_user_id, to_user_id, content, created_at, is_read) VALUES
    -- Alice <-> Bob conversation
    ('750e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440002', 'Hey Bob, love your UI work! 🎨', '2024-01-10T11:00:00Z', 1),
    ('750e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440001', 'Thanks Alice! Your Rust tips are great. 🦀', '2024-01-10T11:05:00Z', 1),
    ('750e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440002', 'Want to pair program on the new feature?', '2024-01-10T11:10:00Z', 1),
    ('750e8400-e29b-41d4-a716-446655440004', '550e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440001', 'Absolutely! When are you free?', '2024-01-10T11:15:00Z', 0),
    
    -- Alice <-> Charlie conversation
    ('750e8400-e29b-41d4-a716-446655440005', '550e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440003', 'Charlie, quick question about SQLite indexing...', '2024-01-09T14:00:00Z', 1),
    ('750e8400-e29b-41d4-a716-446655440006', '550e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440001', 'Sure! What do you need help with?', '2024-01-09T14:05:00Z', 1),
    ('750e8400-e29b-41d4-a716-446655440007', '550e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440003', 'Should I index the created_at column for sorting?', '2024-01-09T14:10:00Z', 1),
    ('750e8400-e29b-41d4-a716-446655440008', '550e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440001', 'Definitely! Especially if you''re sorting frequently. 💾', '2024-01-09T14:15:00Z', 0),
    
    -- Bob <-> Charlie conversation
    ('750e8400-e29b-41d4-a716-446655440009', '550e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440003', 'Hey Charlie, thoughts on the database schema?', '2024-01-08T16:00:00Z', 1),
    ('750e8400-e29b-41d4-a716-446655440010', '550e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440002', 'Looks solid! Foreign keys are properly set up. 👍', '2024-01-08T16:10:00Z', 1),
    ('750e8400-e29b-41d4-a716-446655440011', '550e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440003', 'Great! Should we add more indexes?', '2024-01-08T16:15:00Z', 1),
    ('750e8400-e29b-41d4-a716-446655440012', '550e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440002', 'Let''s wait and see the query patterns first.', '2024-01-08T16:20:00Z', 0);

-- ============================================================================
-- REPLY POSTS (Threaded Conversations)
-- ============================================================================
INSERT OR IGNORE INTO posts (id, author_id, content, created_at, upvotes, downvotes, parent_post_id) VALUES
    -- Replies to Alice's Rust performance post
    ('850e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440002', 'What kind of performance improvements? Share the numbers! 📊', '2024-01-10T10:15:00Z', 8, 0, '650e8400-e29b-41d4-a716-446655440001'),
    ('850e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440008', 'Rust performance is unmatched. Zero-cost abstractions FTW!', '2024-01-10T10:30:00Z', 12, 0, '650e8400-e29b-41d4-a716-446655440001'),
    ('850e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440006', '50% faster than the C version. Rust is incredible 🦀', '2024-01-10T10:45:00Z', 15, 1, '650e8400-e29b-41d4-a716-446655440001'),
    -- Replies to Alice's terminal apps hot take
    ('850e8400-e29b-41d4-a716-446655440004', '550e8400-e29b-41d4-a716-446655440004', 'Terminal apps are making a comeback! Love to see it 🚀', '2024-01-08T09:30:00Z', 11, 0, '650e8400-e29b-41d4-a716-446655440003'),
    ('850e8400-e29b-41d4-a716-446655440005', '550e8400-e29b-41d4-a716-446655440005', 'Electron apps are so bloated. Give me a TUI any day', '2024-01-08T09:45:00Z', 18, 2, '650e8400-e29b-41d4-a716-446655440003'),
    ('850e8400-e29b-41d4-a716-446655440006', '550e8400-e29b-41d4-a716-446655440002', 'Screen readers work with terminals! It''s doable', '2024-01-08T10:15:00Z', 14, 1, '650e8400-e29b-41d4-a716-446655440003'),
    -- Replies to Bob's terminal UI post
    ('850e8400-e29b-41d4-a716-446655440007', '550e8400-e29b-41d4-a716-446655440001', 'Solarized Dark is my go-to. Easy on the eyes 👀', '2024-01-10T08:15:00Z', 10, 0, '650e8400-e29b-41d4-a716-446655440005'),
    ('850e8400-e29b-41d4-a716-446655440008', '550e8400-e29b-41d4-a716-446655440003', 'Nord theme for that cool blue aesthetic ❄️', '2024-01-10T08:45:00Z', 8, 0, '650e8400-e29b-41d4-a716-446655440005'),
    -- Replies to Bob's vim keybindings post
    ('850e8400-e29b-41d4-a716-446655440009', '550e8400-e29b-41d4-a716-446655440001', 'Vim keybindings everywhere! Even in my browser 😂', '2024-01-07T10:45:00Z', 15, 1, '650e8400-e29b-41d4-a716-446655440008'),
    ('850e8400-e29b-41d4-a716-446655440010', '550e8400-e29b-41d4-a716-446655440003', 'Emacs user here. We can coexist peacefully 🤝', '2024-01-07T11:00:00Z', 12, 3, '650e8400-e29b-41d4-a716-446655440008'),
    -- Replies to Charlie's controversial SQLite post
    ('850e8400-e29b-41d4-a716-446655440011', '550e8400-e29b-41d4-a716-446655440001', 'SQLite is perfect for 90% of apps. People overcomplicate things', '2024-01-07T14:30:00Z', 19, 2, '650e8400-e29b-41d4-a716-446655440012'),
    ('850e8400-e29b-41d4-a716-446655440012', '550e8400-e29b-41d4-a716-446655440003', 'Most apps never need horizontal scaling. Vertical scaling works great!', '2024-01-07T15:00:00Z', 16, 3, '650e8400-e29b-41d4-a716-446655440012'),
    -- Replies to Diana's open source post
    ('850e8400-e29b-41d4-a716-446655440013', '550e8400-e29b-41d4-a716-446655440001', 'You''re a legend! Thank you for all your work 🙏', '2024-01-10T12:15:00Z', 22, 0, '650e8400-e29b-41d4-a716-446655440016'),
    ('850e8400-e29b-41d4-a716-446655440014', '550e8400-e29b-41d4-a716-446655440007', 'Maintainers are the backbone of open source ❤️', '2024-01-10T12:30:00Z', 18, 0, '650e8400-e29b-41d4-a716-446655440016'),
    -- Replies to Grace's security CVE post
    ('850e8400-e29b-41d4-a716-446655440015', '550e8400-e29b-41d4-a716-446655440004', 'You''re doing important work! Thank you 🙏', '2024-01-10T14:15:00Z', 25, 0, '650e8400-e29b-41d4-a716-446655440025'),
    ('850e8400-e29b-41d4-a716-446655440016', '550e8400-e29b-41d4-a716-446655440001', 'Security researchers are unsung heroes', '2024-01-10T14:45:00Z', 21, 0, '650e8400-e29b-41d4-a716-446655440025'),
    ('850e8400-e29b-41d4-a716-446655440017', '550e8400-e29b-41d4-a716-446655440007', '9.8 - Remote code execution. Nasty stuff 😱', '2024-01-10T15:15:00Z', 28, 0, '650e8400-e29b-41d4-a716-446655440025'),
    -- Replies to Hank's performance optimization post
    ('850e8400-e29b-41d4-a716-446655440018', '550e8400-e29b-41d4-a716-446655440001', '200ms is huge! What was the bottleneck?', '2024-01-10T11:45:00Z', 10, 0, '650e8400-e29b-41d4-a716-446655440028'),
    ('850e8400-e29b-41d4-a716-446655440019', '550e8400-e29b-41d4-a716-446655440008', 'Memory allocations in the hot path. Classic mistake', '2024-01-10T12:00:00Z', 15, 0, '650e8400-e29b-41d4-a716-446655440028');

-- ============================================================================
-- SOCIAL CONNECTIONS (Follows)
-- ============================================================================
INSERT OR IGNORE INTO follows (follower_id, following_id, created_at) VALUES
    -- Alice follows (active networker)
    ('550e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440002', 1704672000),
    ('550e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440003', 1704672000),
    ('550e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440006', 1704758400),
    ('550e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440008', 1704758400),
    -- Bob follows
    ('550e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440001', 1704672000),
    ('550e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440003', 1704672000),
    ('550e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440004', 1705017600),
    -- Charlie follows
    ('550e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440001', 1704672000),
    ('550e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440002', 1704672000),
    -- Diana follows (popular maintainer)
    ('550e8400-e29b-41d4-a716-446655440004', '550e8400-e29b-41d4-a716-446655440001', 1704758400),
    ('550e8400-e29b-41d4-a716-446655440004', '550e8400-e29b-41d4-a716-446655440007', 1704758400),
    -- Eve follows
    ('550e8400-e29b-41d4-a716-446655440005', '550e8400-e29b-41d4-a716-446655440004', 1704758400),
    ('550e8400-e29b-41d4-a716-446655440005', '550e8400-e29b-41d4-a716-446655440008', 1704844800),
    -- Frank follows
    ('550e8400-e29b-41d4-a716-446655440006', '550e8400-e29b-41d4-a716-446655440001', 1704758400),
    ('550e8400-e29b-41d4-a716-446655440006', '550e8400-e29b-41d4-a716-446655440008', 1704844800),
    -- Grace follows
    ('550e8400-e29b-41d4-a716-446655440007', '550e8400-e29b-41d4-a716-446655440004', 1704758400),
    ('550e8400-e29b-41d4-a716-446655440007', '550e8400-e29b-41d4-a716-446655440001', 1704931200),
    -- Hank follows
    ('550e8400-e29b-41d4-a716-446655440008', '550e8400-e29b-41d4-a716-446655440001', 1704758400),
    ('550e8400-e29b-41d4-a716-446655440008', '550e8400-e29b-41d4-a716-446655440006', 1704758400),
    -- More people follow Alice (she's popular)
    ('550e8400-e29b-41d4-a716-446655440005', '550e8400-e29b-41d4-a716-446655440001', 1705017600),
    -- More people follow Diana (popular maintainer)
    ('550e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440004', 1705017600),
    ('550e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440004', 1705104000),
    ('550e8400-e29b-41d4-a716-446655440006', '550e8400-e29b-41d4-a716-446655440004', 1705190400),
    ('550e8400-e29b-41d4-a716-446655440008', '550e8400-e29b-41d4-a716-446655440004', 1705276800);
"#;
