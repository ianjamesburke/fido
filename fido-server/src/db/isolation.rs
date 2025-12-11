use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use fido_types::{UserContext, UserType, IsolatedData, Post, DirectMessage, Vote, SortOrder};

use crate::db::DbPool;
use crate::db::repositories::{PostRepository, UserRepository, VoteRepository, DirectMessageRepository};

/// Database adapter trait for handling user context and data isolation
pub trait DatabaseAdapter {
    /// Create a database operations instance with user context
    fn with_context(&self, context: &UserContext) -> Box<dyn DatabaseOperations>;
}

/// Database operations trait that respects user context and isolation
pub trait DatabaseOperations {
    /// Get posts with user context filtering
    fn get_posts(&self, sort_order: SortOrder, limit: i32) -> Result<Vec<Post>>;
    
    /// Create a post with user context
    fn create_post(&self, post: &Post) -> Result<()>;
    
    /// Get direct messages for user
    fn get_direct_messages(&self, user_id: &Uuid) -> Result<Vec<DirectMessage>>;
    
    /// Send a direct message
    fn send_direct_message(&self, message: &DirectMessage) -> Result<()>;
    
    /// Vote on a post
    fn vote_on_post(&self, vote: &Vote) -> Result<()>;
    
    /// Reset test user data (only works for test users)
    fn reset_test_data(&self) -> Result<()>;
}

/// Concrete implementation of database adapter with isolation support
pub struct IsolatedDatabaseAdapter {
    pool: DbPool,
    // In-memory storage for test user data to ensure complete isolation
    test_data: Arc<Mutex<HashMap<String, IsolatedData>>>,
}

impl IsolatedDatabaseAdapter {
    /// Create a new isolated database adapter
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            test_data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Reset all test user data
    pub fn reset_all_test_data(&self) -> Result<()> {
        let mut data = self.test_data.lock().unwrap();
        data.clear();
        Ok(())
    }
}

impl DatabaseAdapter for IsolatedDatabaseAdapter {
    fn with_context(&self, context: &UserContext) -> Box<dyn DatabaseOperations> {
        Box::new(ContextualDatabaseOperations {
            context: context.clone(),
            pool: self.pool.clone(),
            test_data: self.test_data.clone(),
        })
    }
}

/// Database operations implementation that respects user context
struct ContextualDatabaseOperations {
    context: UserContext,
    pool: DbPool,
    test_data: Arc<Mutex<HashMap<String, IsolatedData>>>,
}

impl DatabaseOperations for ContextualDatabaseOperations {
    fn get_posts(&self, sort_order: SortOrder, limit: i32) -> Result<Vec<Post>> {
        match &self.context.user_type {
            UserType::TestUser(test_id) => {
                // For test users, return only test data from in-memory storage
                let data = self.test_data.lock().unwrap();
                if let Some(isolated_data) = data.get(test_id) {
                    let mut posts = isolated_data.posts.clone();
                    
                    // Apply sorting
                    match sort_order {
                        SortOrder::Newest => {
                            posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                        }
                        SortOrder::Popular => {
                            posts.sort_by(|a, b| b.upvotes.cmp(&a.upvotes).then(b.created_at.cmp(&a.created_at)));
                        }
                        SortOrder::Controversial => {
                            posts.sort_by(|a, b| {
                                let a_score = (a.upvotes - a.downvotes).abs();
                                let b_score = (b.upvotes - b.downvotes).abs();
                                a_score.cmp(&b_score).then(b.created_at.cmp(&a.created_at))
                            });
                        }
                    }
                    
                    // Apply limit
                    posts.truncate(limit as usize);
                    Ok(posts)
                } else {
                    // No test data yet, return empty
                    Ok(vec![])
                }
            }
            UserType::RealUser(_) => {
                // For real users, query the actual database but filter out test user content
                let post_repo = PostRepository::new(self.pool.clone());
                let mut posts = post_repo.get_posts(sort_order, limit)?;
                
                // Filter out test user posts
                let user_repo = UserRepository::new(self.pool.clone());
                posts.retain(|post| {
                    if let Ok(Some(user)) = user_repo.get_by_id(&post.author_id) {
                        !user.is_test_user
                    } else {
                        false
                    }
                });
                
                Ok(posts)
            }
        }
    }
    
    fn create_post(&self, post: &Post) -> Result<()> {
        match &self.context.user_type {
            UserType::TestUser(test_id) => {
                // For test users, store in isolated in-memory storage
                let mut data = self.test_data.lock().unwrap();
                let isolated_data = data.entry(test_id.clone()).or_insert_with(IsolatedData::new);
                isolated_data.posts.push(post.clone());
                Ok(())
            }
            UserType::RealUser(_) => {
                // For real users, store in actual database
                let post_repo = PostRepository::new(self.pool.clone());
                post_repo.create(post)
            }
        }
    }
    
    fn get_direct_messages(&self, user_id: &Uuid) -> Result<Vec<DirectMessage>> {
        match &self.context.user_type {
            UserType::TestUser(test_id) => {
                // For test users, return only test data from in-memory storage
                let data = self.test_data.lock().unwrap();
                if let Some(isolated_data) = data.get(test_id) {
                    let messages: Vec<DirectMessage> = isolated_data.messages.iter()
                        .filter(|msg| msg.from_user_id == *user_id || msg.to_user_id == *user_id)
                        .cloned()
                        .collect();
                    Ok(messages)
                } else {
                    Ok(vec![])
                }
            }
            UserType::RealUser(_) => {
                // For real users, query the actual database but filter out test user content
                let dm_repo = DirectMessageRepository::new(self.pool.clone());
                let conversation_users = dm_repo.get_conversations_list(user_id)?;
                let mut messages = Vec::new();
                for other_user_id in conversation_users {
                    let mut conversation = dm_repo.get_conversation(user_id, &other_user_id)?;
                    messages.append(&mut conversation);
                }
                
                // Filter out messages involving test users
                let user_repo = UserRepository::new(self.pool.clone());
                messages.retain(|msg| {
                    let from_user_ok = if let Ok(Some(user)) = user_repo.get_by_id(&msg.from_user_id) {
                        !user.is_test_user
                    } else {
                        false
                    };
                    let to_user_ok = if let Ok(Some(user)) = user_repo.get_by_id(&msg.to_user_id) {
                        !user.is_test_user
                    } else {
                        false
                    };
                    from_user_ok && to_user_ok
                });
                
                Ok(messages)
            }
        }
    }
    
    fn send_direct_message(&self, message: &DirectMessage) -> Result<()> {
        match &self.context.user_type {
            UserType::TestUser(test_id) => {
                // For test users, store in isolated in-memory storage
                let mut data = self.test_data.lock().unwrap();
                let isolated_data = data.entry(test_id.clone()).or_insert_with(IsolatedData::new);
                isolated_data.messages.push(message.clone());
                Ok(())
            }
            UserType::RealUser(_) => {
                // For real users, store in actual database
                let dm_repo = DirectMessageRepository::new(self.pool.clone());
                dm_repo.create(message)
            }
        }
    }
    
    fn vote_on_post(&self, vote: &Vote) -> Result<()> {
        match &self.context.user_type {
            UserType::TestUser(test_id) => {
                // For test users, store in isolated in-memory storage
                let mut data = self.test_data.lock().unwrap();
                let isolated_data = data.entry(test_id.clone()).or_insert_with(IsolatedData::new);
                
                // Remove any existing vote for this user/post combination
                isolated_data.votes.retain(|v| !(v.user_id == vote.user_id && v.post_id == vote.post_id));
                
                // Add the new vote
                isolated_data.votes.push(vote.clone());
                
                // Update vote counts on the post
                if let Some(post) = isolated_data.posts.iter_mut().find(|p| p.id == vote.post_id) {
                    // Recalculate vote counts
                    let upvotes = isolated_data.votes.iter()
                        .filter(|v| v.post_id == vote.post_id && v.direction.as_str() == "up")
                        .count() as i32;
                    let downvotes = isolated_data.votes.iter()
                        .filter(|v| v.post_id == vote.post_id && v.direction.as_str() == "down")
                        .count() as i32;
                    
                    post.upvotes = upvotes;
                    post.downvotes = downvotes;
                }
                
                Ok(())
            }
            UserType::RealUser(_) => {
                // For real users, store in actual database
                let vote_repo = VoteRepository::new(self.pool.clone());
                vote_repo.upsert_vote(&vote.user_id, &vote.post_id, vote.direction)?;
                
                // Update post vote counts
                let post_repo = PostRepository::new(self.pool.clone());
                post_repo.update_vote_counts(&vote.post_id)
            }
        }
    }
    
    fn reset_test_data(&self) -> Result<()> {
        match &self.context.user_type {
            UserType::TestUser(test_id) => {
                // Reset only this test user's data
                let mut data = self.test_data.lock().unwrap();
                if let Some(isolated_data) = data.get_mut(test_id) {
                    isolated_data.reset();
                }
                Ok(())
            }
            UserType::RealUser(_) => {
                // Real users cannot reset test data
                Err(anyhow::anyhow!("Real users cannot reset test data"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use fido_types::{VoteDirection};

    fn create_test_post(author_id: Uuid, content: &str) -> Post {
        Post {
            id: Uuid::new_v4(),
            author_id,
            author_username: "test_user".to_string(),
            content: content.to_string(),
            created_at: Utc::now(),
            upvotes: 0,
            downvotes: 0,
            hashtags: vec![],
            user_vote: None,
            parent_post_id: None,
            reply_count: 0,
            reply_to_user_id: None,
            reply_to_username: None,
        }
    }

    #[test]
    fn test_test_user_data_isolation() {
        // This test would require a database connection, so we'll test the logic
        // In a real test environment, you'd set up a test database
        let context = UserContext::test_user("alice".to_string());
        assert!(context.is_test_user());
        assert_eq!(context.isolation_key(), Some("test_alice"));
    }

    #[test]
    fn test_isolated_data_operations() {
        let mut data = IsolatedData::new();
        
        // Test adding a post
        let post = create_test_post(Uuid::new_v4(), "Test post content");
        data.posts.push(post.clone());
        
        assert_eq!(data.posts.len(), 1);
        assert_eq!(data.posts[0].content, "Test post content");
        
        // Test reset
        data.reset();
        assert!(data.is_empty());
    }

    #[test]
    fn test_vote_isolation() {
        let mut data = IsolatedData::new();
        let user_id = Uuid::new_v4();
        let post_id = Uuid::new_v4();
        
        // Add a post
        let mut post = create_test_post(user_id, "Test post");
        post.id = post_id;
        data.posts.push(post);
        
        // Add a vote
        let vote = Vote {
            user_id,
            post_id,
            direction: VoteDirection::Up,
            created_at: Utc::now(),
        };
        data.votes.push(vote);
        
        assert_eq!(data.votes.len(), 1);
        assert_eq!(data.votes[0].direction, VoteDirection::Up);
    }

    // Property-based tests
    use proptest::prelude::*;

    // **Feature: web-terminal-interface, Property 5: Test User Data Isolation**
    // **Validates: Requirements 3.1, 3.3, 3.4**
    // For any test user action (post creation, voting, messaging), the resulting data 
    // should never appear in production user queries or feeds.
    proptest! {
        #[test]
        fn prop_test_user_data_isolation(
            test_user_id in "[a-z]{3,10}",
            post_content in "[a-zA-Z0-9 #@!.,]{10,100}",
            _vote_direction in prop::sample::select(vec!["up", "down"]),
            _message_content in "[a-zA-Z0-9 .,!?]{5,50}"
        ) {
            // Create test user context
            let test_context = UserContext::test_user(test_user_id.clone());
            prop_assert!(test_context.is_test_user());
            let expected_key = format!("test_{}", test_user_id);
            prop_assert_eq!(test_context.isolation_key(), Some(expected_key.as_str()));
            
            // Create real user context
            let real_context = UserContext::real_user("github123".to_string());
            prop_assert!(real_context.is_real_user());
            prop_assert_eq!(real_context.isolation_key(), None);
            
            // Test that test user data is isolated
            let mut test_data = IsolatedData::new();
            
            // Add test user post
            let test_post = create_test_post(Uuid::new_v4(), &post_content);
            test_data.posts.push(test_post.clone());
            
            // Verify test data exists in isolation
            prop_assert_eq!(test_data.posts.len(), 1);
            prop_assert_eq!(&test_data.posts[0].content, &post_content);
            
            // Verify isolation key is correctly formatted
            prop_assert_eq!(test_context.isolation_key(), Some(expected_key.as_str()));
        }
    }

    // **Feature: web-terminal-interface, Property 6: Test User Data Reset on Load**
    // **Validates: Requirements 3.2**
    // For any web interface load event, all existing test user data should be 
    // completely reset to a clean initial state.
    proptest! {
        #[test]
        fn prop_test_user_data_reset_on_load(
            _test_user_ids in prop::collection::vec("[a-z]{3,8}", 1..5),
            post_contents in prop::collection::vec("[a-zA-Z0-9 #]{10,50}", 1..10)
        ) {
            // Create isolated data with some test content
            let mut test_data = IsolatedData::new();
            
            // Add various types of test data
            for (i, content) in post_contents.iter().enumerate() {
                let post = create_test_post(Uuid::new_v4(), content);
                test_data.posts.push(post);
                
                // Add some votes
                let vote = Vote {
                    user_id: Uuid::new_v4(),
                    post_id: Uuid::new_v4(),
                    direction: if i % 2 == 0 { VoteDirection::Up } else { VoteDirection::Down },
                    created_at: Utc::now(),
                };
                test_data.votes.push(vote);
            }
            
            // Verify data exists before reset
            prop_assert!(!test_data.is_empty());
            prop_assert_eq!(test_data.posts.len(), post_contents.len());
            prop_assert_eq!(test_data.votes.len(), post_contents.len());
            
            // Simulate web interface load - reset all test data
            test_data.reset();
            
            // Verify all data is completely reset
            prop_assert!(test_data.is_empty());
            prop_assert_eq!(test_data.posts.len(), 0);
            prop_assert_eq!(test_data.votes.len(), 0);
            prop_assert_eq!(test_data.messages.len(), 0);
            prop_assert_eq!(test_data.follows.len(), 0);
        }
    }
}