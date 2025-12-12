use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User context system for test user isolation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserContext {
    pub user_type: UserType,
    pub isolation_key: Option<String>,
}

/// Enum to distinguish between test and real users
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserType {
    /// Real authenticated GitHub user
    RealUser(String), // GitHub user ID
    /// Test user for demonstration purposes
    TestUser(String), // Test user identifier
}

impl UserContext {
    /// Create a new user context for a real user
    pub fn real_user(github_id: String) -> Self {
        Self {
            user_type: UserType::RealUser(github_id),
            isolation_key: None,
        }
    }

    /// Create a new user context for a test user
    pub fn test_user(test_id: String) -> Self {
        let isolation_key = Some(format!("test_{}", test_id));
        Self {
            user_type: UserType::TestUser(test_id),
            isolation_key,
        }
    }

    /// Check if this is a test user
    pub fn is_test_user(&self) -> bool {
        matches!(self.user_type, UserType::TestUser(_))
    }

    /// Check if this is a real user
    pub fn is_real_user(&self) -> bool {
        matches!(self.user_type, UserType::RealUser(_))
    }

    /// Get the user identifier
    pub fn user_id(&self) -> &str {
        match &self.user_type {
            UserType::RealUser(id) => id,
            UserType::TestUser(id) => id,
        }
    }

    /// Get the isolation key for data separation
    pub fn isolation_key(&self) -> Option<&str> {
        self.isolation_key.as_deref()
    }
}

/// Isolated data container for test users
#[derive(Debug, Clone, Default)]
pub struct IsolatedData {
    pub posts: Vec<crate::models::Post>,
    pub messages: Vec<crate::models::DirectMessage>,
    pub votes: Vec<crate::models::Vote>,
    pub follows: Vec<Uuid>, // Following relationships
}

impl IsolatedData {
    /// Create a new empty isolated data container
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all data to clean state
    pub fn reset(&mut self) {
        self.posts.clear();
        self.messages.clear();
        self.votes.clear();
        self.follows.clear();
    }

    /// Check if the data container is empty
    pub fn is_empty(&self) -> bool {
        self.posts.is_empty()
            && self.messages.is_empty()
            && self.votes.is_empty()
            && self.follows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_user_context() {
        let context = UserContext::real_user("github123".to_string());

        assert!(context.is_real_user());
        assert!(!context.is_test_user());
        assert_eq!(context.user_id(), "github123");
        assert_eq!(context.isolation_key(), None);
    }

    #[test]
    fn test_test_user_context() {
        let context = UserContext::test_user("alice".to_string());

        assert!(context.is_test_user());
        assert!(!context.is_real_user());
        assert_eq!(context.user_id(), "alice");
        assert_eq!(context.isolation_key(), Some("test_alice"));
    }

    #[test]
    fn test_isolated_data() {
        let mut data = IsolatedData::new();

        assert!(data.is_empty());

        // Add some mock data
        data.posts.push(crate::models::Post {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            author_username: "test".to_string(),
            content: "test post".to_string(),
            created_at: chrono::Utc::now(),
            upvotes: 0,
            downvotes: 0,
            hashtags: vec![],
            user_vote: None,
            parent_post_id: None,
            reply_count: 0,
            reply_to_user_id: None,
            reply_to_username: None,
        });

        assert!(!data.is_empty());

        data.reset();
        assert!(data.is_empty());
    }
}
