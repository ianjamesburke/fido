use axum::{extract::State, Json};

use crate::{
    api::{ApiError, ApiResult},
    http::AuthenticatedUser,
    state::AppState,
};
use fido_types::{ColorScheme, SortOrder, UpdateConfigRequest, UserConfig};

/// GET /config - Get user configuration
pub async fn get_config(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<Json<UserConfig>> {
    // Get configuration (returns default if not found)
    let config = state
        .repos
        .config
        .get(&user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(config))
}

/// PUT /config - Update user configuration
pub async fn update_config(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<UpdateConfigRequest>,
) -> ApiResult<Json<UserConfig>> {
    // Get current config
    let mut config = state
        .repos
        .config
        .get(&user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Update fields if provided
    if let Some(color_scheme_str) = payload.color_scheme {
        let color_scheme = ColorScheme::parse(&color_scheme_str).ok_or_else(|| {
            ApiError::BadRequest(format!(
                "Invalid color scheme '{}'. Valid options: Default, Dark, Light, Solarized",
                color_scheme_str
            ))
        })?;
        config.color_scheme = color_scheme;
    }

    if let Some(sort_order_str) = payload.sort_order {
        let sort_order = SortOrder::parse(&sort_order_str).ok_or_else(|| {
            ApiError::BadRequest(format!(
                "Invalid sort order '{}'. Valid options: Newest, Popular, Controversial",
                sort_order_str
            ))
        })?;
        config.sort_order = sort_order;
    }

    if let Some(max_posts) = payload.max_posts_display {
        // Validate max_posts > 0
        if max_posts <= 0 {
            return Err(ApiError::BadRequest(
                "max_posts_display must be greater than 0".to_string(),
            ));
        }
        config.max_posts_display = max_posts;
    }

    if let Some(emoji_enabled) = payload.emoji_enabled {
        config.emoji_enabled = emoji_enabled;
    }

    // Save updated config
    state
        .repos
        .config
        .update(&config)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(config))
}
