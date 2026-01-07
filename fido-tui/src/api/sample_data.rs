use chrono::{Duration, Utc};
use uuid::Uuid;
use fido_types::*;

/// Create test users for demo mode
pub fn create_test_users() -> Vec<User> {
    let now = Utc::now();
    
    vec![
        User {
            id: Uuid::new_v4(),
            username: "demo_user".to_string(),
            bio: Some("Welcome to Fido! This is the default demo account.".to_string()),
            join_date: now,
            is_test_user: true,
        },
        User {
            id: Uuid::new_v4(),
            username: "alice".to_string(),
            bio: Some("Rust enthusiast 🦀 | Building fast, safe systems".to_string()),
            join_date: now,
            is_test_user: true,
        },
        User {
            id: Uuid::new_v4(),
            username: "bob".to_string(),
            bio: Some("Terminal lover 💻 | Vim or die".to_string()),
            join_date: now,
            is_test_user: true,
        },
        User {
            id: Uuid::new_v4(),
            username: "charlie".to_string(),
            bio: Some("Open source contributor 🌍 | Coffee-driven development".to_string()),
            join_date: now,
            is_test_user: true,
        },
    ]
}

/// Create sample posts with hashtags and replies
pub fn create_sample_posts(users: &[User]) -> Vec<Post> {
    let now = Utc::now();
    let mut posts = Vec::new();
    
    // Helper to find user by username
    let find_user = |username: &str| -> &User {
        users.iter().find(|u| u.username == username).unwrap()
    };
    
    // Top-level posts (15-20 posts)
    
    // Post 1 - alice
    let post1_id = Uuid::new_v4();
    posts.push(Post {
        id: post1_id,
        author_id: find_user("alice").id,
        author_username: "alice".to_string(),
        content: "Just discovered #fido - this terminal social network is amazing! 🦀 #rust".to_string(),
        created_at: now - Duration::hours(48),
        upvotes: 42,
        downvotes: 2,
        hashtags: vec!["fido".to_string(), "rust".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 2,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 2 - bob
    let post2_id = Uuid::new_v4();
    posts.push(Post {
        id: post2_id,
        author_id: find_user("bob").id,
        author_username: "bob".to_string(),
        content: "Who else loves working in the #terminal? No distractions, just pure productivity. #coding".to_string(),
        created_at: now - Duration::hours(36),
        upvotes: 28,
        downvotes: 1,
        hashtags: vec!["terminal".to_string(), "coding".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 1,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 3 - charlie
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("charlie").id,
        author_username: "charlie".to_string(),
        content: "Contributing to #opensource projects is so rewarding. Just merged my first PR to a #rust project!".to_string(),
        created_at: now - Duration::hours(30),
        upvotes: 35,
        downvotes: 0,
        hashtags: vec!["opensource".to_string(), "rust".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 4 - demo_user
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("demo_user").id,
        author_username: "demo_user".to_string(),
        content: "Hello Fido! First post from the demo account. Excited to explore this platform! #fido".to_string(),
        created_at: now - Duration::hours(24),
        upvotes: 15,
        downvotes: 0,
        hashtags: vec!["fido".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 5 - alice
    let post5_id = Uuid::new_v4();
    posts.push(Post {
        id: post5_id,
        author_id: find_user("alice").id,
        author_username: "alice".to_string(),
        content: "Debugging #rust async code at 2am. Send coffee ☕ #coding".to_string(),
        created_at: now - Duration::hours(20),
        upvotes: 50,
        downvotes: 3,
        hashtags: vec!["rust".to_string(), "coding".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 3,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 6 - bob
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("bob").id,
        author_username: "bob".to_string(),
        content: "Just switched to a tiling window manager. My productivity has doubled! #terminal".to_string(),
        created_at: now - Duration::hours(18),
        upvotes: 22,
        downvotes: 5,
        hashtags: vec!["terminal".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 7 - charlie
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("charlie").id,
        author_username: "charlie".to_string(),
        content: "Looking for #opensource projects to contribute to. Any recommendations? #rust #coding".to_string(),
        created_at: now - Duration::hours(16),
        upvotes: 18,
        downvotes: 1,
        hashtags: vec!["opensource".to_string(), "rust".to_string(), "coding".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 8 - demo_user
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("demo_user").id,
        author_username: "demo_user".to_string(),
        content: "The keyboard shortcuts in #fido are so intuitive. Love the #terminal UI!".to_string(),
        created_at: now - Duration::hours(14),
        upvotes: 12,
        downvotes: 0,
        hashtags: vec!["fido".to_string(), "terminal".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 9 - alice
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("alice").id,
        author_username: "alice".to_string(),
        content: "Memory safety without garbage collection. That's why I love #rust 🦀".to_string(),
        created_at: now - Duration::hours(12),
        upvotes: 45,
        downvotes: 2,
        hashtags: vec!["rust".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 10 - bob
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("bob").id,
        author_username: "bob".to_string(),
        content: "Vim keybindings everywhere. Even in my browser. #terminal #coding".to_string(),
        created_at: now - Duration::hours(10),
        upvotes: 30,
        downvotes: 8,
        hashtags: vec!["terminal".to_string(), "coding".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 11 - charlie
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("charlie").id,
        author_username: "charlie".to_string(),
        content: "Hacktoberfest is coming! Time to contribute to #opensource projects 🎃 #coding".to_string(),
        created_at: now - Duration::hours(8),
        upvotes: 25,
        downvotes: 0,
        hashtags: vec!["opensource".to_string(), "coding".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 12 - demo_user
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("demo_user").id,
        author_username: "demo_user".to_string(),
        content: "Learning #rust has been challenging but so worth it! #coding".to_string(),
        created_at: now - Duration::hours(6),
        upvotes: 20,
        downvotes: 1,
        hashtags: vec!["rust".to_string(), "coding".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 13 - alice
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("alice").id,
        author_username: "alice".to_string(),
        content: "Just released my first crate on crates.io! 🎉 #rust #opensource".to_string(),
        created_at: now - Duration::hours(5),
        upvotes: 38,
        downvotes: 0,
        hashtags: vec!["rust".to_string(), "opensource".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 14 - bob
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("bob").id,
        author_username: "bob".to_string(),
        content: "The #terminal is not just a tool, it's a lifestyle. #coding".to_string(),
        created_at: now - Duration::hours(4),
        upvotes: 16,
        downvotes: 2,
        hashtags: vec!["terminal".to_string(), "coding".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 15 - charlie
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("charlie").id,
        author_username: "charlie".to_string(),
        content: "Code review tip: Be kind, be constructive, be helpful. #opensource #coding".to_string(),
        created_at: now - Duration::hours(3),
        upvotes: 33,
        downvotes: 0,
        hashtags: vec!["opensource".to_string(), "coding".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 16 - demo_user
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("demo_user").id,
        author_username: "demo_user".to_string(),
        content: "This demo mode is pretty cool! All data is ephemeral. #fido".to_string(),
        created_at: now - Duration::hours(2),
        upvotes: 10,
        downvotes: 0,
        hashtags: vec!["fido".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 17 - alice
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("alice").id,
        author_username: "alice".to_string(),
        content: "Cargo makes dependency management so easy. Love the #rust ecosystem! 📦".to_string(),
        created_at: now - Duration::hours(1),
        upvotes: 27,
        downvotes: 1,
        hashtags: vec!["rust".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 18 - bob
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("bob").id,
        author_username: "bob".to_string(),
        content: "Tmux + Vim + Fido = Perfect development environment 💯 #terminal".to_string(),
        created_at: now - Duration::minutes(45),
        upvotes: 19,
        downvotes: 3,
        hashtags: vec!["terminal".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 19 - charlie
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("charlie").id,
        author_username: "charlie".to_string(),
        content: "Documentation is code. Write it well. #opensource #coding".to_string(),
        created_at: now - Duration::minutes(30),
        upvotes: 24,
        downvotes: 0,
        hashtags: vec!["opensource".to_string(), "coding".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Post 20 - demo_user
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("demo_user").id,
        author_username: "demo_user".to_string(),
        content: "Exploring all the features of #fido. This is fun! #terminal".to_string(),
        created_at: now - Duration::minutes(15),
        upvotes: 8,
        downvotes: 0,
        hashtags: vec!["fido".to_string(), "terminal".to_string()],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    });
    
    // Now add replies (5-10 replies threaded under some posts)
    
    // Reply 1 to post1 (alice's fido discovery) - from bob
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("bob").id,
        author_username: "bob".to_string(),
        content: "Welcome! The keyboard shortcuts are amazing once you get used to them.".to_string(),
        created_at: now - Duration::hours(47),
        upvotes: 8,
        downvotes: 0,
        hashtags: vec![],
        user_vote: None,
        parent_post_id: Some(post1_id),
        reply_count: 0,
        reply_to_user_id: Some(find_user("alice").id),
        reply_to_username: Some("alice".to_string()),
    });
    
    // Reply 2 to post1 - from charlie
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("charlie").id,
        author_username: "charlie".to_string(),
        content: "Agreed! No ads, no algorithms, just pure content. #fido".to_string(),
        created_at: now - Duration::hours(46),
        upvotes: 12,
        downvotes: 0,
        hashtags: vec!["fido".to_string()],
        user_vote: None,
        parent_post_id: Some(post1_id),
        reply_count: 0,
        reply_to_user_id: Some(find_user("alice").id),
        reply_to_username: Some("alice".to_string()),
    });
    
    // Reply 3 to post2 (bob's terminal love) - from alice
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("alice").id,
        author_username: "alice".to_string(),
        content: "Same here! I do everything in the terminal now. So much faster.".to_string(),
        created_at: now - Duration::hours(35),
        upvotes: 5,
        downvotes: 0,
        hashtags: vec![],
        user_vote: None,
        parent_post_id: Some(post2_id),
        reply_count: 0,
        reply_to_user_id: Some(find_user("bob").id),
        reply_to_username: Some("bob".to_string()),
    });
    
    // Reply 4 to post5 (alice's debugging) - from demo_user
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("demo_user").id,
        author_username: "demo_user".to_string(),
        content: "Been there! Async can be tricky. Have you tried tokio-console?".to_string(),
        created_at: now - Duration::hours(19),
        upvotes: 6,
        downvotes: 0,
        hashtags: vec![],
        user_vote: None,
        parent_post_id: Some(post5_id),
        reply_count: 0,
        reply_to_user_id: Some(find_user("alice").id),
        reply_to_username: Some("alice".to_string()),
    });
    
    // Reply 5 to post5 - from bob
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("bob").id,
        author_username: "bob".to_string(),
        content: "Coffee is the answer to all debugging problems ☕".to_string(),
        created_at: now - Duration::hours(18),
        upvotes: 15,
        downvotes: 0,
        hashtags: vec![],
        user_vote: None,
        parent_post_id: Some(post5_id),
        reply_count: 0,
        reply_to_user_id: Some(find_user("alice").id),
        reply_to_username: Some("alice".to_string()),
    });
    
    // Reply 6 to post5 - from charlie
    posts.push(Post {
        id: Uuid::new_v4(),
        author_id: find_user("charlie").id,
        author_username: "charlie".to_string(),
        content: "The compiler errors in #rust are actually helpful though!".to_string(),
        created_at: now - Duration::hours(17),
        upvotes: 10,
        downvotes: 1,
        hashtags: vec!["rust".to_string()],
        user_vote: None,
        parent_post_id: Some(post5_id),
        reply_count: 0,
        reply_to_user_id: Some(find_user("alice").id),
        reply_to_username: Some("alice".to_string()),
    });
    
    posts
}

/// Create sample DM conversations
pub fn create_sample_conversations(users: &[User]) -> Vec<DirectMessage> {
    let now = Utc::now();
    let mut messages = Vec::new();
    
    // Helper to find user by username
    let find_user = |username: &str| -> &User {
        users.iter().find(|u| u.username == username).unwrap()
    };
    
    // Conversation 1: demo_user <-> alice (4 messages)
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("alice").id,
        to_user_id: find_user("demo_user").id,
        from_username: "alice".to_string(),
        to_username: "demo_user".to_string(),
        content: "Hey! Welcome to Fido! Let me know if you have any questions.".to_string(),
        created_at: now - Duration::hours(24),
        is_read: true,
    });
    
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("demo_user").id,
        to_user_id: find_user("alice").id,
        from_username: "demo_user".to_string(),
        to_username: "alice".to_string(),
        content: "Thanks! This platform is really cool. How do I filter posts by hashtag?".to_string(),
        created_at: now - Duration::hours(23),
        is_read: true,
    });
    
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("alice").id,
        to_user_id: find_user("demo_user").id,
        from_username: "alice".to_string(),
        to_username: "demo_user".to_string(),
        content: "Just press 'f' to open the filter modal, then type the hashtag you want!".to_string(),
        created_at: now - Duration::hours(22),
        is_read: true,
    });
    
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("demo_user").id,
        to_user_id: find_user("alice").id,
        from_username: "demo_user".to_string(),
        to_username: "alice".to_string(),
        content: "Perfect, got it! The keyboard shortcuts are so intuitive.".to_string(),
        created_at: now - Duration::hours(21),
        is_read: true,
    });
    
    // Conversation 2: demo_user <-> bob (3 messages)
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("demo_user").id,
        to_user_id: find_user("bob").id,
        from_username: "demo_user".to_string(),
        to_username: "bob".to_string(),
        content: "I saw your post about tiling window managers. Which one do you use?".to_string(),
        created_at: now - Duration::hours(12),
        is_read: true,
    });
    
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("bob").id,
        to_user_id: find_user("demo_user").id,
        from_username: "bob".to_string(),
        to_username: "demo_user".to_string(),
        content: "I'm using i3wm. It's amazing once you configure it properly!".to_string(),
        created_at: now - Duration::hours(11),
        is_read: true,
    });
    
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("bob").id,
        to_user_id: find_user("demo_user").id,
        from_username: "bob".to_string(),
        to_username: "demo_user".to_string(),
        content: "Let me know if you want to see my config files!".to_string(),
        created_at: now - Duration::hours(11),
        is_read: false,
    });
    
    // Conversation 3: demo_user <-> charlie (5 messages)
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("charlie").id,
        to_user_id: find_user("demo_user").id,
        from_username: "charlie".to_string(),
        to_username: "demo_user".to_string(),
        content: "I noticed you're learning Rust! That's awesome!".to_string(),
        created_at: now - Duration::hours(8),
        is_read: true,
    });
    
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("demo_user").id,
        to_user_id: find_user("charlie").id,
        from_username: "demo_user".to_string(),
        to_username: "charlie".to_string(),
        content: "Yeah! It's challenging but I'm enjoying it. Any tips?".to_string(),
        created_at: now - Duration::hours(7),
        is_read: true,
    });
    
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("charlie").id,
        to_user_id: find_user("demo_user").id,
        from_username: "charlie".to_string(),
        to_username: "demo_user".to_string(),
        content: "Start with small projects. The Rust book is excellent. And don't fight the borrow checker!".to_string(),
        created_at: now - Duration::hours(6),
        is_read: true,
    });
    
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("demo_user").id,
        to_user_id: find_user("charlie").id,
        from_username: "demo_user".to_string(),
        to_username: "charlie".to_string(),
        content: "Haha, the borrow checker and I are still getting acquainted 😅".to_string(),
        created_at: now - Duration::hours(5),
        is_read: true,
    });
    
    messages.push(DirectMessage {
        id: Uuid::new_v4(),
        from_user_id: find_user("charlie").id,
        to_user_id: find_user("demo_user").id,
        from_username: "charlie".to_string(),
        to_username: "demo_user".to_string(),
        content: "You'll get there! Feel free to ask if you get stuck on anything.".to_string(),
        created_at: now - Duration::hours(4),
        is_read: false,
    });
    
    messages
}
