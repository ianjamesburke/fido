use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::db::repositories::UserRepository;
use crate::state::AppState;
use fido_types::UserContext;

/// Extension type for user context that gets added to requests
#[derive(Clone, Debug)]
pub struct RequestUserContext {
    pub user_context: UserContext,
    pub user_id: Uuid,
    pub is_authenticated: bool,
}

impl RequestUserContext {
    pub fn new(user_context: UserContext, user_id: Uuid) -> Self {
        Self {
            user_context,
            user_id,
            is_authenticated: true,
        }
    }

    pub fn unauthenticated() -> Self {
        Self {
            user_context: UserContext::test_user("anonymous".to_string()),
            user_id: Uuid::nil(),
            is_authenticated: false,
        }
    }
}

/// Middleware to detect and inject user context into requests
///
/// This middleware examines the session token and determines the user context
/// (test user vs real user) for proper data isolation.
pub async fn user_context_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();

    // Try to extract session token
    let session_token = headers.get("X-Session-Token").and_then(|v| v.to_str().ok());

    let user_context = if let Some(token) = session_token {
        // Validate session and get user context
        match get_user_context_from_token(&state, token).await {
            Ok(context) => context,
            Err(_) => {
                // Invalid session, treat as unauthenticated
                RequestUserContext::unauthenticated()
            }
        }
    } else {
        // No session token, treat as unauthenticated
        RequestUserContext::unauthenticated()
    };

    // Add user context to request extensions
    request.extensions_mut().insert(user_context);

    next.run(request).await
}

/// Helper function to get user context from session token
async fn get_user_context_from_token(
    state: &AppState,
    token: &str,
) -> Result<RequestUserContext, Box<dyn std::error::Error + Send + Sync>> {
    // Validate session
    let user_id = state.session_manager.validate_session(token)?;

    // Get user information
    let repo = UserRepository::new(state.db.pool.clone());
    let user = repo.get_by_id(&user_id)?.ok_or("User not found")?;

    // Create appropriate user context
    let user_context = if user.is_test_user {
        UserContext::test_user(user.username.clone())
    } else {
        let github_login = user.github_login.ok_or("Real user missing GitHub login")?;
        UserContext::real_user(github_login)
    };

    Ok(RequestUserContext::new(user_context, user_id))
}

/// Helper function to extract user context from request
pub fn extract_user_context(headers: &HeaderMap) -> Option<RequestUserContext> {
    // This would be populated by the middleware
    // For now, we'll implement a simple version that checks headers
    None
}

/// Middleware to ensure only authenticated users can access certain endpoints
pub async fn require_auth_middleware(request: Request, next: Next) -> Response {
    // Check if user context indicates authentication
    if let Some(context) = request.extensions().get::<RequestUserContext>() {
        if context.is_authenticated {
            return next.run(request).await;
        }
    }

    // Return unauthorized response
    axum::response::Response::builder()
        .status(401)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            r#"{"error":"Authentication required"}"#,
        ))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_user_context_creation() {
        let user_id = Uuid::new_v4();
        let user_context = UserContext::test_user("alice".to_string());

        let request_context = RequestUserContext::new(user_context.clone(), user_id);

        assert!(request_context.is_authenticated);
        assert_eq!(request_context.user_id, user_id);
        assert_eq!(request_context.user_context, user_context);
    }

    #[test]
    fn test_unauthenticated_context() {
        let context = RequestUserContext::unauthenticated();

        assert!(!context.is_authenticated);
        assert_eq!(context.user_id, Uuid::nil());
        assert!(context.user_context.is_test_user());
    }
}
