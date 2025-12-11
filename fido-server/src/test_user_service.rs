use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use fido_types::{Post, UserContext, IsolatedData};
use crate::db::{DbPool, IsolatedDatabaseAdapter, DatabaseAdapter};

/// Service for managing test user data and isolation
pub struct TestUserService {
    db_adapter: IsolatedDatabaseAdapter,
}

impl TestUserService {
    /// Create a new test user service
    pub fn new(pool: DbPool) -> Self {
        Self {
            db_adapter: IsolatedDatabaseAdapter::new(pool),
        }
    }
    
    /// Reset all test user data to clean state
    pub fn reset_all_test_data(&self) -> Result<()> {
        self.db_adapter.reset_all_test_data()
    }
    
    /// Reset specific test user data
    pub fn reset_test_user_data(&self, test_user_id: &str) -> Result<()> {
        let context = UserContext::test_user(test_user_id.to_string());
        let db_ops = self.db_adapter.with_context(&context);
        db_ops.reset_test_data()
    }
    
    /// Initialize default test user data
    pub fn initialize_test_data(&self) -> Result<()> {
        // Reset all test data first
        self.reset_all_test_data()?;
        
        // Create some sample test posts for demonstration
        let test_users = vec![
            ("alice", "Rust enthusiast and terminal lover 🦀"),
            ("bob", "Terminal UI designer and developer 🎨"),
            ("charlie", "SQLite advocate and database expert 💾"),
        ];
        
        for (username, _bio) in test_users {
            let context = UserContext::test_user(username.to_string());
            let db_ops = self.db_adapter.with_context(&context);
            
            // Create some sample posts for each test user
            let sample_posts = match username {
                "alice" => vec![
                    "Just shipped a new #rust feature! 🚀 The performance improvements are incredible.",
                    "Learning about #async #rust patterns. The borrow checker is my friend! 💪",
                    "Hot take: #terminal apps are the future of developer tools. No bloat, just speed. ⚡",
                ],
                "bob" => vec![
                    "Working on #terminal #ui design. Any tips for better color schemes? 🎨",
                    "Just discovered #crossterm for terminal manipulation. Game changer! 🔥",
                    "Keyboard-driven interfaces are so much faster than mouse-based UIs. #productivity",
                ],
                "charlie" => vec![
                    "Love the simplicity of #sqlite for MVPs. Perfect for rapid prototyping! 💡",
                    "#sqlite is underrated. It powers more apps than you think! 📱",
                    "Database indexing 101: Always index your foreign keys! #database #performance",
                ],
                _ => vec![],
            };
            
            for (i, content) in sample_posts.iter().enumerate() {
                let post = Post {
                    id: Uuid::new_v4(),
                    author_id: Uuid::new_v4(), // Test user ID
                    author_username: username.to_string(),
                    content: content.to_string(),
                    created_at: Utc::now() - chrono::Duration::minutes(i as i64 * 30),
                    upvotes: (i + 1) as i32 * 3, // Some variety in votes
                    downvotes: if i % 3 == 0 { 1 } else { 0 },
                    hashtags: vec![], // Will be extracted from content
                    user_vote: None,
                    parent_post_id: None,
                    reply_count: 0,
                    reply_to_user_id: None,
                    reply_to_username: None,
                };
                
                db_ops.create_post(&post)?;
            }
        }
        
        Ok(())
    }
    
    /// Get database adapter for use with user contexts
    pub fn get_database_adapter(&self) -> &IsolatedDatabaseAdapter {
        &self.db_adapter
    }
    
    /// Check if a user context represents a test user
    pub fn is_test_user_context(&self, context: &UserContext) -> bool {
        context.is_test_user()
    }
    
    /// Filter out test user content from results (for real users)
    pub fn filter_test_user_content<T>(&self, items: Vec<T>, is_test_item: impl Fn(&T) -> bool) -> Vec<T> {
        items.into_iter().filter(|item| !is_test_item(item)).collect()
    }
}

/// Test user data reset trigger - called when web interface loads
pub struct TestUserResetTrigger {
    service: TestUserService,
}

impl TestUserResetTrigger {
    /// Create a new reset trigger
    pub fn new(pool: DbPool) -> Self {
        Self {
            service: TestUserService::new(pool),
        }
    }
    
    /// Trigger test data reset and initialization
    /// This should be called when the web interface loads
    pub fn trigger_reset(&self) -> Result<()> {
        log::info!("Triggering test user data reset");
        self.service.reset_all_test_data()?;
        self.service.initialize_test_data()?;
        log::info!("Test user data reset and initialized successfully");
        Ok(())
    }
    
    /// Get the underlying service
    pub fn service(&self) -> &TestUserService {
        &self.service
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_context_creation() {
        let real_context = UserContext::real_user("github123".to_string());
        let test_context = UserContext::test_user("alice".to_string());
        
        assert!(real_context.is_real_user());
        assert!(!real_context.is_test_user());
        
        assert!(test_context.is_test_user());
        assert!(!test_context.is_real_user());
        assert_eq!(test_context.isolation_key(), Some("test_alice"));
    }

    #[test]
    fn test_isolated_data_operations() {
        let mut data = IsolatedData::new();
        assert!(data.is_empty());
        
        // Add a mock post
        let post = Post {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            author_username: "test".to_string(),
            content: "test content".to_string(),
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
        
        data.posts.push(post);
        assert!(!data.is_empty());
        
        data.reset();
        assert!(data.is_empty());
    }

    #[test]
    fn test_filter_test_content() {
        let items = vec![
            ("real_item", false),
            ("test_item", true),
            ("another_real", false),
        ];
        
        let filtered: Vec<_> = items.into_iter()
            .filter(|(_, is_test)| !is_test)
            .collect();
        
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].0, "real_item");
        assert_eq!(filtered[1].0, "another_real");
    }
}