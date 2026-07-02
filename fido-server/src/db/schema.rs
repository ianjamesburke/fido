/// SQL schema for the Fido database
/// Creates all tables with proper constraints, foreign keys, and indexes
pub const SCHEMA: &str = r#"
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    bio TEXT,
    join_date TEXT NOT NULL,
    is_test_user INTEGER NOT NULL DEFAULT 0,
    is_admin INTEGER NOT NULL DEFAULT 0
);

-- Communities table (keyed to GitHub repositories)
CREATE TABLE IF NOT EXISTS communities (
    id TEXT PRIMARY KEY,
    github_repo_id INTEGER UNIQUE NOT NULL,
    owner TEXT NOT NULL,
    name TEXT NOT NULL,
    claimed_by TEXT,
    require_thread_approval INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (claimed_by) REFERENCES users(id) ON DELETE SET NULL
);

-- Channels table (chat channels, scoped per community)
CREATE TABLE IF NOT EXISTS channels (
    id TEXT PRIMARY KEY,
    community_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (community_id, name),
    FOREIGN KEY (community_id) REFERENCES communities(id) ON DELETE CASCADE
);

-- Messages table (append-only chat messages)
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for cursor pagination on (channel_id, id)
CREATE INDEX IF NOT EXISTS idx_messages_channel_id ON messages(channel_id, id);

-- Posts table
CREATE TABLE IF NOT EXISTS posts (
    id TEXT PRIMARY KEY,
    author_id TEXT NOT NULL,
    community_id TEXT NOT NULL,
    content TEXT NOT NULL CHECK(length(content) <= 280),
    created_at TEXT NOT NULL,
    upvotes INTEGER NOT NULL DEFAULT 0,
    downvotes INTEGER NOT NULL DEFAULT 0,
    approved INTEGER NOT NULL DEFAULT 1,
    parent_post_id TEXT,
    reply_to_user_id TEXT,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (community_id) REFERENCES communities(id),
    FOREIGN KEY (parent_post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (reply_to_user_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Create index on created_at for efficient post sorting
CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at DESC);

-- Create index on parent_post_id for efficient reply lookups
CREATE INDEX IF NOT EXISTS idx_posts_parent_post_id ON posts(parent_post_id);

-- Create index on community_id for efficient community-scoped lookups
CREATE INDEX IF NOT EXISTS idx_posts_community_id ON posts(community_id);

-- Memberships table (community membership with role)
CREATE TABLE IF NOT EXISTS memberships (
    community_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('admin', 'contributor', 'member')),
    created_at TEXT NOT NULL,
    PRIMARY KEY (community_id, user_id),
    FOREIGN KEY (community_id) REFERENCES communities(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Notifications table (generic "something happened to you" pipeline)
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    type TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (actor_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for unread notification counts per user
CREATE INDEX IF NOT EXISTS idx_notifications_user_read ON notifications(user_id, read);

-- DM conversations table (server-enforced message requests)
CREATE TABLE IF NOT EXISTS dm_conversations (
    user_a TEXT NOT NULL,
    user_b TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending', 'accepted', 'declined')),
    initiator_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (user_a, user_b),
    CHECK (user_a < user_b),
    FOREIGN KEY (user_a) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (user_b) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (initiator_id) REFERENCES users(id) ON DELETE CASCADE
);

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

-- GitHub OAuth tokens (encrypted at rest)
CREATE TABLE IF NOT EXISTS github_tokens (
    user_id TEXT PRIMARY KEY,
    encrypted_token TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Post rate limiting table
CREATE TABLE IF NOT EXISTS post_rate_limits (
    user_id TEXT PRIMARY KEY,
    last_post_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- DM rate limiting table
CREATE TABLE IF NOT EXISTS dm_rate_limits (
    user_id TEXT PRIMARY KEY,
    last_dm_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create index for efficient rate limit lookups
CREATE INDEX IF NOT EXISTS idx_post_rate_limits_user ON post_rate_limits(user_id);
CREATE INDEX IF NOT EXISTS idx_dm_rate_limits_user ON dm_rate_limits(user_id);

-- Audit logs table for security event tracking
CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    user_id TEXT,
    ip_address TEXT,
    user_agent TEXT,
    details TEXT,
    timestamp TEXT NOT NULL
);

-- Indexes for efficient audit log queries
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type ON audit_logs(event_type);
"#;

/// Test data for development and testing
/// Includes comprehensive test data for all features:
/// - 3 test users (alice, bob, charlie)
/// - A seed community with a #general channel and memberships
/// - Multiple posts scoped to the seed community
/// - Votes demonstrating upvote/downvote functionality
/// - Direct message conversations between users
/// - User configurations with different preferences
pub const TEST_DATA: &str = r#"
-- ============================================================================
-- TEST USERS
-- ============================================================================
-- Insert test users with diverse profiles
-- Note: alice is set as admin (is_admin = 1) for testing admin functionality
INSERT OR IGNORE INTO users (id, username, bio, join_date, is_test_user, is_admin) VALUES
    ('550e8400-e29b-41d4-a716-446655440001', 'alice', 'Rust enthusiast and terminal lover 🦀', '2024-01-01T00:00:00Z', 1, 1),
    ('550e8400-e29b-41d4-a716-446655440002', 'bob', 'Terminal UI designer and developer 🎨', '2024-01-02T00:00:00Z', 1, 0),
    ('550e8400-e29b-41d4-a716-446655440003', 'charlie', 'SQLite advocate and database expert 💾', '2024-01-03T00:00:00Z', 1, 0),
    ('550e8400-e29b-41d4-a716-446655440004', 'diana', 'Open source maintainer | Coffee addict ☕', '2024-01-04T00:00:00Z', 1, 0),
    ('550e8400-e29b-41d4-a716-446655440005', 'eve', 'DevOps engineer | Automation enthusiast 🤖', '2024-01-05T00:00:00Z', 1, 0),
    ('550e8400-e29b-41d4-a716-446655440006', 'frank', 'Systems programmer | Low-level wizard ⚡', '2024-01-06T00:00:00Z', 1, 0),
    ('550e8400-e29b-41d4-a716-446655440007', 'grace', 'Security researcher | Bug bounty hunter 🔒', '2024-01-07T00:00:00Z', 1, 0),
    ('550e8400-e29b-41d4-a716-446655440008', 'hank', 'Performance optimization nerd | Benchmarking 📊', '2024-01-08T00:00:00Z', 1, 0);

-- ============================================================================
-- COMMUNITY, CHANNEL, MEMBERSHIPS
-- ============================================================================
-- Seed community that owns all sample posts, plus its default #general channel.
INSERT OR IGNORE INTO communities (id, github_repo_id, owner, name, claimed_by, require_thread_approval, created_at) VALUES
    ('950e8400-e29b-41d4-a716-446655440001', 1, 'ianjamesburke', 'fido', '550e8400-e29b-41d4-a716-446655440001', 0, '2024-01-01T00:00:00Z');

INSERT OR IGNORE INTO channels (id, community_id, name, created_at) VALUES
    ('a50e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'general', '2024-01-01T00:00:00Z');

INSERT OR IGNORE INTO memberships (community_id, user_id, role, created_at) VALUES
    ('950e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440001', 'admin', '2024-01-01T00:00:00Z'),
    ('950e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440002', 'member', '2024-01-02T00:00:00Z'),
    ('950e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440003', 'member', '2024-01-03T00:00:00Z'),
    ('950e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440004', 'member', '2024-01-04T00:00:00Z'),
    ('950e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440005', 'member', '2024-01-05T00:00:00Z'),
    ('950e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440006', 'member', '2024-01-06T00:00:00Z'),
    ('950e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440007', 'member', '2024-01-07T00:00:00Z'),
    ('950e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440008', 'member', '2024-01-08T00:00:00Z');

-- ============================================================================
-- USER CONFIGURATIONS
-- ============================================================================
-- Insert default configs for test users with different preferences
INSERT OR IGNORE INTO user_configs (user_id, color_scheme, sort_order, max_posts_display, emoji_enabled) VALUES
    ('550e8400-e29b-41d4-a716-446655440001', 'Dark', 'Newest', 25, 1),
    ('550e8400-e29b-41d4-a716-446655440002', 'Dark', 'Popular', 50, 1),
    ('550e8400-e29b-41d4-a716-446655440003', 'Dark', 'Newest', 30, 1),
    ('550e8400-e29b-41d4-a716-446655440004', 'Dark', 'Popular', 40, 1),
    ('550e8400-e29b-41d4-a716-446655440005', 'Dark', 'Newest', 35, 1),
    ('550e8400-e29b-41d4-a716-446655440006', 'Dark', 'Popular', 20, 1),
    ('550e8400-e29b-41d4-a716-446655440007', 'Dark', 'Newest', 50, 1),
    ('550e8400-e29b-41d4-a716-446655440008', 'Dark', 'Popular', 25, 1);

-- ============================================================================
-- SAMPLE POSTS
-- ============================================================================
-- Insert diverse posts with emojis and content, all scoped to the seed community
INSERT OR IGNORE INTO posts (id, author_id, community_id, content, created_at, upvotes, downvotes) VALUES
    ('650e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'Just shipped a new #rust feature! 🚀 The performance improvements are incredible.', '2024-01-10T10:00:00Z', 15, 1),
    ('650e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'Learning about #async #rust patterns. The borrow checker is my friend! 💪', '2024-01-09T14:30:00Z', 8, 0),
    ('650e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'Hot take: #terminal apps are the future of developer tools. No bloat, just speed. ⚡', '2024-01-08T09:15:00Z', 23, 3),
    ('650e8400-e29b-41d4-a716-446655440004', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'Anyone else obsessed with #ratatui? Building TUIs has never been easier! 🎯', '2024-01-07T16:45:00Z', 12, 0),
    ('650e8400-e29b-41d4-a716-446655440005', '550e8400-e29b-41d4-a716-446655440002', '950e8400-e29b-41d4-a716-446655440001', 'Working on #terminal #ui design. Any tips for better color schemes? 🎨', '2024-01-10T08:00:00Z', 18, 2),
    ('650e8400-e29b-41d4-a716-446655440006', '550e8400-e29b-41d4-a716-446655440002', '950e8400-e29b-41d4-a716-446655440001', 'Just discovered #crossterm for terminal manipulation. Game changer! 🔥', '2024-01-09T11:20:00Z', 10, 0),
    ('650e8400-e29b-41d4-a716-446655440007', '550e8400-e29b-41d4-a716-446655440002', '950e8400-e29b-41d4-a716-446655440001', 'Keyboard-driven interfaces are so much faster than mouse-based UIs. #productivity #terminal', '2024-01-08T13:00:00Z', 14, 1),
    ('650e8400-e29b-41d4-a716-446655440008', '550e8400-e29b-41d4-a716-446655440002', '950e8400-e29b-41d4-a716-446655440001', 'Pro tip: Use vim keybindings everywhere. Your fingers will thank you! ⌨️ #vim #productivity', '2024-01-07T10:30:00Z', 20, 4),
    ('650e8400-e29b-41d4-a716-446655440009', '550e8400-e29b-41d4-a716-446655440003', '950e8400-e29b-41d4-a716-446655440001', 'Love the simplicity of #sqlite for MVPs. Perfect for rapid prototyping! 💡', '2024-01-10T06:00:00Z', 16, 0),
    ('650e8400-e29b-41d4-a716-446655440010', '550e8400-e29b-41d4-a716-446655440003', '950e8400-e29b-41d4-a716-446655440001', '#sqlite is underrated. It powers more apps than you think! 📱', '2024-01-09T15:45:00Z', 11, 1),
    ('650e8400-e29b-41d4-a716-446655440011', '550e8400-e29b-41d4-a716-446655440003', '950e8400-e29b-41d4-a716-446655440001', 'Database indexing 101: Always index your foreign keys! #database #performance', '2024-01-08T12:00:00Z', 9, 0),
    ('650e8400-e29b-41d4-a716-446655440012', '550e8400-e29b-41d4-a716-446655440003', '950e8400-e29b-41d4-a716-446655440001', 'Controversial opinion: Not every app needs PostgreSQL. #sqlite #database', '2024-01-07T14:20:00Z', 25, 8),
    ('650e8400-e29b-41d4-a716-446655440013', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'Shoutout to @bob for the amazing #ui work on Fido! 🙌', '2024-01-06T17:00:00Z', 19, 0),
    ('650e8400-e29b-41d4-a716-446655440014', '550e8400-e29b-41d4-a716-446655440002', '950e8400-e29b-41d4-a716-446655440001', 'Thanks @alice! Couldn''t have done it without your #rust expertise. 🤝', '2024-01-06T17:15:00Z', 15, 0),
    ('650e8400-e29b-41d4-a716-446655440015', '550e8400-e29b-41d4-a716-446655440003', '950e8400-e29b-41d4-a716-446655440001', 'This team is crushing it! #fido #rust #terminal #collaboration', '2024-01-06T17:30:00Z', 22, 0),
    ('650e8400-e29b-41d4-a716-446655440016', '550e8400-e29b-41d4-a716-446655440004', '950e8400-e29b-41d4-a716-446655440001', 'Just merged 50 PRs today! Open source never sleeps 😅 #opensource #maintainer', '2024-01-10T12:00:00Z', 28, 2),
    ('650e8400-e29b-41d4-a716-446655440017', '550e8400-e29b-41d4-a716-446655440004', '950e8400-e29b-41d4-a716-446655440001', 'Coffee count: 7 ☕ Bug fixes: 12 🐛 Good day! #coding #coffee', '2024-01-09T18:00:00Z', 21, 0),
    ('650e8400-e29b-41d4-a716-446655440018', '550e8400-e29b-41d4-a716-446655440004', '950e8400-e29b-41d4-a716-446655440001', 'Remember: Good documentation is love for your future self ❤️ #documentation #opensource', '2024-01-08T11:00:00Z', 35, 1),
    ('650e8400-e29b-41d4-a716-446655440019', '550e8400-e29b-41d4-a716-446655440005', '950e8400-e29b-41d4-a716-446655440001', 'Automated all the things! CI/CD pipeline running smooth 🚀 #devops #automation', '2024-01-10T09:30:00Z', 24, 0),
    ('650e8400-e29b-41d4-a716-446655440020', '550e8400-e29b-41d4-a716-446655440005', '950e8400-e29b-41d4-a716-446655440001', 'Infrastructure as code is the only way. Fight me. 🤖 #terraform #kubernetes', '2024-01-09T13:00:00Z', 31, 5),
    ('650e8400-e29b-41d4-a716-446655440021', '550e8400-e29b-41d4-a716-446655440005', '950e8400-e29b-41d4-a716-446655440001', 'Monitoring dashboards looking beautiful today 📊 #observability #metrics', '2024-01-08T15:30:00Z', 18, 0),
    ('650e8400-e29b-41d4-a716-446655440022', '550e8400-e29b-41d4-a716-446655440006', '950e8400-e29b-41d4-a716-446655440001', 'Writing assembly for fun on a Friday night ⚡ #lowlevel #systems', '2024-01-10T20:00:00Z', 16, 3),
    ('650e8400-e29b-41d4-a716-446655440023', '550e8400-e29b-41d4-a716-446655440006', '950e8400-e29b-41d4-a716-446655440001', 'Memory safety without garbage collection? That''s why I love #rust 🦀', '2024-01-09T10:00:00Z', 27, 1),
    ('650e8400-e29b-41d4-a716-446655440024', '550e8400-e29b-41d4-a716-446655440006', '950e8400-e29b-41d4-a716-446655440001', 'Kernel hacking is meditation for programmers 🧘 #linux #kernel', '2024-01-08T08:00:00Z', 22, 2),
    ('650e8400-e29b-41d4-a716-446655440025', '550e8400-e29b-41d4-a716-446655440007', '950e8400-e29b-41d4-a716-446655440001', 'Found another critical CVE today 🔒 Responsible disclosure in progress #security #bugbounty', '2024-01-10T14:00:00Z', 42, 0),
    ('650e8400-e29b-41d4-a716-446655440026', '550e8400-e29b-41d4-a716-446655440007', '950e8400-e29b-41d4-a716-446655440001', 'PSA: Always sanitize your inputs! SQL injection is still a thing in 2024 😱 #security #appsec', '2024-01-09T16:30:00Z', 38, 1),
    ('650e8400-e29b-41d4-a716-446655440027', '550e8400-e29b-41d4-a716-446655440007', '950e8400-e29b-41d4-a716-446655440001', 'Fuzzing is my favorite pastime 🎯 #security #fuzzing #testing', '2024-01-08T19:00:00Z', 19, 0),
    ('650e8400-e29b-41d4-a716-446655440028', '550e8400-e29b-41d4-a716-446655440008', '950e8400-e29b-41d4-a716-446655440001', 'Shaved 200ms off the hot path! Benchmarks don''t lie 📊 #performance #optimization', '2024-01-10T11:30:00Z', 33, 0),
    ('650e8400-e29b-41d4-a716-446655440029', '550e8400-e29b-41d4-a716-446655440008', '950e8400-e29b-41d4-a716-446655440001', 'Premature optimization is the root of all evil, but profiling is divine ⚡ #performance', '2024-01-09T12:00:00Z', 29, 2),
    ('650e8400-e29b-41d4-a716-446655440030', '550e8400-e29b-41d4-a716-446655440008', '950e8400-e29b-41d4-a716-446655440001', 'Cache invalidation: the hardest problem in computer science 🤔 #caching #performance', '2024-01-08T14:00:00Z', 26, 1)
;

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
INSERT OR IGNORE INTO posts (id, author_id, community_id, content, created_at, upvotes, downvotes, parent_post_id) VALUES
    ('850e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440002', '950e8400-e29b-41d4-a716-446655440001', 'What kind of performance improvements? Share the numbers! 📊', '2024-01-10T10:15:00Z', 8, 0, '650e8400-e29b-41d4-a716-446655440001'),
    ('850e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440008', '950e8400-e29b-41d4-a716-446655440001', 'Rust performance is unmatched. Zero-cost abstractions FTW!', '2024-01-10T10:30:00Z', 12, 0, '650e8400-e29b-41d4-a716-446655440001'),
    ('850e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440006', '950e8400-e29b-41d4-a716-446655440001', '50% faster than the C version. Rust is incredible 🦀', '2024-01-10T10:45:00Z', 15, 1, '650e8400-e29b-41d4-a716-446655440001'),
    ('850e8400-e29b-41d4-a716-446655440004', '550e8400-e29b-41d4-a716-446655440004', '950e8400-e29b-41d4-a716-446655440001', 'Terminal apps are making a comeback! Love to see it 🚀', '2024-01-08T09:30:00Z', 11, 0, '650e8400-e29b-41d4-a716-446655440003'),
    ('850e8400-e29b-41d4-a716-446655440005', '550e8400-e29b-41d4-a716-446655440005', '950e8400-e29b-41d4-a716-446655440001', 'Electron apps are so bloated. Give me a TUI any day', '2024-01-08T09:45:00Z', 18, 2, '650e8400-e29b-41d4-a716-446655440003'),
    ('850e8400-e29b-41d4-a716-446655440006', '550e8400-e29b-41d4-a716-446655440002', '950e8400-e29b-41d4-a716-446655440001', 'Screen readers work with terminals! It''s doable', '2024-01-08T10:15:00Z', 14, 1, '650e8400-e29b-41d4-a716-446655440003'),
    ('850e8400-e29b-41d4-a716-446655440007', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'Solarized Dark is my go-to. Easy on the eyes 👀', '2024-01-10T08:15:00Z', 10, 0, '650e8400-e29b-41d4-a716-446655440005'),
    ('850e8400-e29b-41d4-a716-446655440008', '550e8400-e29b-41d4-a716-446655440003', '950e8400-e29b-41d4-a716-446655440001', 'Nord theme for that cool blue aesthetic ❄️', '2024-01-10T08:45:00Z', 8, 0, '650e8400-e29b-41d4-a716-446655440005'),
    ('850e8400-e29b-41d4-a716-446655440009', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'Vim keybindings everywhere! Even in my browser 😂', '2024-01-07T10:45:00Z', 15, 1, '650e8400-e29b-41d4-a716-446655440008'),
    ('850e8400-e29b-41d4-a716-446655440010', '550e8400-e29b-41d4-a716-446655440003', '950e8400-e29b-41d4-a716-446655440001', 'Emacs user here. We can coexist peacefully 🤝', '2024-01-07T11:00:00Z', 12, 3, '650e8400-e29b-41d4-a716-446655440008'),
    ('850e8400-e29b-41d4-a716-446655440011', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'SQLite is perfect for 90% of apps. People overcomplicate things', '2024-01-07T14:30:00Z', 19, 2, '650e8400-e29b-41d4-a716-446655440012'),
    ('850e8400-e29b-41d4-a716-446655440012', '550e8400-e29b-41d4-a716-446655440003', '950e8400-e29b-41d4-a716-446655440001', 'Most apps never need horizontal scaling. Vertical scaling works great!', '2024-01-07T15:00:00Z', 16, 3, '650e8400-e29b-41d4-a716-446655440012'),
    ('850e8400-e29b-41d4-a716-446655440013', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'You''re a legend! Thank you for all your work 🙏', '2024-01-10T12:15:00Z', 22, 0, '650e8400-e29b-41d4-a716-446655440016'),
    ('850e8400-e29b-41d4-a716-446655440014', '550e8400-e29b-41d4-a716-446655440007', '950e8400-e29b-41d4-a716-446655440001', 'Maintainers are the backbone of open source ❤️', '2024-01-10T12:30:00Z', 18, 0, '650e8400-e29b-41d4-a716-446655440016'),
    ('850e8400-e29b-41d4-a716-446655440015', '550e8400-e29b-41d4-a716-446655440004', '950e8400-e29b-41d4-a716-446655440001', 'You''re doing important work! Thank you 🙏', '2024-01-10T14:15:00Z', 25, 0, '650e8400-e29b-41d4-a716-446655440025'),
    ('850e8400-e29b-41d4-a716-446655440016', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', 'Security researchers are unsung heroes', '2024-01-10T14:45:00Z', 21, 0, '650e8400-e29b-41d4-a716-446655440025'),
    ('850e8400-e29b-41d4-a716-446655440017', '550e8400-e29b-41d4-a716-446655440007', '950e8400-e29b-41d4-a716-446655440001', '9.8 - Remote code execution. Nasty stuff 😱', '2024-01-10T15:15:00Z', 28, 0, '650e8400-e29b-41d4-a716-446655440025'),
    ('850e8400-e29b-41d4-a716-446655440018', '550e8400-e29b-41d4-a716-446655440001', '950e8400-e29b-41d4-a716-446655440001', '200ms is huge! What was the bottleneck?', '2024-01-10T11:45:00Z', 10, 0, '650e8400-e29b-41d4-a716-446655440028'),
    ('850e8400-e29b-41d4-a716-446655440019', '550e8400-e29b-41d4-a716-446655440008', '950e8400-e29b-41d4-a716-446655440001', 'Memory allocations in the hot path. Classic mistake', '2024-01-10T12:00:00Z', 15, 0, '650e8400-e29b-41d4-a716-446655440028')
;

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
