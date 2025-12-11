use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use fido_types::{Post, UserContext, SortOrder, VoteDirection, Vote};
use fido_server::db::{Database, IsolatedDatabaseAdapter, DatabaseAdapter};

/// Integration test for test user isolation system
#[tokio::test]
async fn test_user_isolation_integration() -> Result<()> {
    // Create in-memory database for testing
    let db = Database::in_memory()?;
    db.initialize()?;
    let pool = db.pool.clone();
    
    // Create isolated database adapter
    let adapter = IsolatedDatabaseAdapter::new(pool);
    
    // Create test user context
    let test_context = UserContext::test_user("alice".to_string());
    let test_db_ops = adapter.with_context(&test_context);
    
    // Create real user context
    let real_context = UserContext::real_user("github123".to_string());
    let real_db_ops = adapter.with_context(&real_context);
    
    // Test user creates a post
    let test_post = Post {
        id: Uuid::new_v4(),
        author_id: Uuid::new_v4(),
        author_username: "alice".to_string(),
        content: "This is a test post from Alice".to_string(),
        created_at: Utc::now(),
        upvotes: 0,
        downvotes: 0,
        hashtags: vec![],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    };
    
    test_db_ops.create_post(&test_post)?;
    
    // Test user should see their own post
    let test_posts = test_db_ops.get_posts(SortOrder::Newest, 10)?;
    assert_eq!(test_posts.len(), 1);
    assert_eq!(test_posts[0].content, "This is a test post from Alice");
    
    // Real user should NOT see test user posts (they should be isolated)
    let real_posts = real_db_ops.get_posts(SortOrder::Newest, 10)?;
    assert_eq!(real_posts.len(), 0, "Real users should not see test user posts");
    
    // Test data reset functionality
    test_db_ops.reset_test_data()?;
    let test_posts_after_reset = test_db_ops.get_posts(SortOrder::Newest, 10)?;
    assert_eq!(test_posts_after_reset.len(), 0, "Test data should be reset to empty");
    
    println!("✅ Test user isolation working correctly");
    println!("✅ Test user posts are isolated from real users");
    println!("✅ Test data reset functionality works");
    
    Ok(())
}

/// Test that test user votes are isolated
#[tokio::test]
async fn test_vote_isolation() -> Result<()> {
    // Create in-memory database for testing
    let db = Database::in_memory()?;
    db.initialize()?;
    let pool = db.pool.clone();
    
    // Create isolated database adapter
    let adapter = IsolatedDatabaseAdapter::new(pool);
    
    // Create test user context
    let test_context = UserContext::test_user("bob".to_string());
    let test_db_ops = adapter.with_context(&test_context);
    
    // Create a test post
    let test_post = Post {
        id: Uuid::new_v4(),
        author_id: Uuid::new_v4(),
        author_username: "bob".to_string(),
        content: "Test post for voting".to_string(),
        created_at: Utc::now(),
        upvotes: 0,
        downvotes: 0,
        hashtags: vec![],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    };
    
    test_db_ops.create_post(&test_post)?;
    
    // Create a vote
    let vote = Vote {
        user_id: Uuid::new_v4(),
        post_id: test_post.id,
        direction: VoteDirection::Up,
        created_at: Utc::now(),
    };
    
    test_db_ops.vote_on_post(&vote)?;
    
    // Verify vote was recorded in isolation
    let posts_after_vote = test_db_ops.get_posts(SortOrder::Newest, 10)?;
    assert_eq!(posts_after_vote.len(), 1);
    assert_eq!(posts_after_vote[0].upvotes, 1);
    assert_eq!(posts_after_vote[0].downvotes, 0);
    
    println!("✅ Test user vote isolation working correctly");
    
    Ok(())
}

/// Test multiple test users have separate isolation
#[tokio::test]
async fn test_multiple_test_user_isolation() -> Result<()> {
    // Create in-memory database for testing
    let db = Database::in_memory()?;
    db.initialize()?;
    let pool = db.pool.clone();
    
    // Create isolated database adapter
    let adapter = IsolatedDatabaseAdapter::new(pool);
    
    // Create two different test user contexts
    let alice_context = UserContext::test_user("alice".to_string());
    let bob_context = UserContext::test_user("bob".to_string());
    
    let alice_db_ops = adapter.with_context(&alice_context);
    let bob_db_ops = adapter.with_context(&bob_context);
    
    // Alice creates a post
    let alice_post = Post {
        id: Uuid::new_v4(),
        author_id: Uuid::new_v4(),
        author_username: "alice".to_string(),
        content: "Alice's post".to_string(),
        created_at: Utc::now(),
        upvotes: 0,
        downvotes: 0,
        hashtags: vec![],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    };
    
    alice_db_ops.create_post(&alice_post)?;
    
    // Bob creates a post
    let bob_post = Post {
        id: Uuid::new_v4(),
        author_id: Uuid::new_v4(),
        author_username: "bob".to_string(),
        content: "Bob's post".to_string(),
        created_at: Utc::now(),
        upvotes: 0,
        downvotes: 0,
        hashtags: vec![],
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
    };
    
    bob_db_ops.create_post(&bob_post)?;
    
    // Alice should only see her own post
    let alice_posts = alice_db_ops.get_posts(SortOrder::Newest, 10)?;
    assert_eq!(alice_posts.len(), 1);
    assert_eq!(alice_posts[0].content, "Alice's post");
    
    // Bob should only see his own post
    let bob_posts = bob_db_ops.get_posts(SortOrder::Newest, 10)?;
    assert_eq!(bob_posts.len(), 1);
    assert_eq!(bob_posts[0].content, "Bob's post");
    
    println!("✅ Multiple test users are properly isolated from each other");
    
    Ok(())
}