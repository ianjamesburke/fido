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

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("X-Session-Token")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::Unauthorized("Missing session token".to_string()))?;

        let user_id = state
            .get_authenticated_user_id_from_token(token)
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
        let token = parts
            .headers
            .get("X-Session-Token")
            .and_then(|v| v.to_str().ok());

        if let Some(token) = token {
            let user_id = state.get_authenticated_user_id_from_token(token);
            return Ok(OptionalUser(user_id));
        }

        Ok(OptionalUser(None))
    }
}
