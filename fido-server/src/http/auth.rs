//! Auth-related request extractors.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::api::ApiError;
use crate::state::AppState;

/// Authenticated user extracted from a valid session token.
pub struct AuthenticatedUser(pub Uuid);

/// Optional authenticated user for endpoints where auth is optional.
pub struct OptionalUser(pub Option<Uuid>);

/// Resolve the session token from the request.
///
/// Prefers the explicit `X-Session-Token` header (the native TUI's path), then
/// falls back to the `session_token` cookie the login handler sets. The cookie
/// is `HttpOnly; SameSite=Strict`, so accepting it here means a browser client
/// never has to hold the raw token in JS-accessible storage, and `SameSite=Strict`
/// keeps it CSRF-safe.
fn session_token_from_parts(parts: &Parts) -> Option<String> {
    if let Some(header) = parts
        .headers
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok())
    {
        return Some(header.to_string());
    }

    parts
        .headers
        .get("Cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(session_cookie_value)
}

/// Extract the `session_token` value from a `Cookie` header, if present.
fn session_cookie_value(cookie_header: &str) -> Option<String> {
    cookie_header.split(';').find_map(|pair| {
        pair.trim()
            .strip_prefix("session_token=")
            .map(str::to_string)
    })
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = session_token_from_parts(parts)
            .ok_or_else(|| ApiError::Unauthorized("Missing session token".to_string()))?;

        let user_id = state
            .get_authenticated_user_id_from_token(&token)
            .ok_or_else(|| ApiError::Unauthorized("Invalid session token".to_string()))?;

        Ok(AuthenticatedUser(user_id))
    }
}

#[async_trait]
impl FromRequestParts<AppState> for OptionalUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(token) = session_token_from_parts(parts) {
            let user_id = state.get_authenticated_user_id_from_token(&token);
            return Ok(OptionalUser(user_id));
        }

        Ok(OptionalUser(None))
    }
}

#[cfg(test)]
mod tests {
    use super::session_cookie_value;

    #[test]
    fn extracts_session_token_cookie() {
        assert_eq!(
            session_cookie_value("session_token=abc123").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            session_cookie_value("theme=dark; session_token=xyz; other=1").as_deref(),
            Some("xyz")
        );
        assert_eq!(session_cookie_value("theme=dark; other=1"), None);
    }
}
