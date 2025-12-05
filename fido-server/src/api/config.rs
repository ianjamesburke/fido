use axum::{extract::State, Json};
use uuid::Uuid;

use crate::{
    api::{ApiError, ApiResult},
    db::repositories::ConfigRepository,
    state::AppState,
};
use fido_types::{ColorScheme, SortOrder, UpdateConfigRequest, UserConfig};

/// GET /config - Get user configuration
pub async fn get_config(State(state): State<AppState>) -> ApiResult<Json<UserConfig>> {
    // For MVP, we'll use a hardcoded user ID (alice)
    // In production, this would come from the authenticated session
    let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001")
        .expect("Invalid hardcoded UUID");

    let pool = state.db.pool.clone();
    let config_repo = ConfigRepository::new(pool);

    // Get configuration (returns default if not found)
    let config = config_repo
        .get(&user_id)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(config))
}

/// PUT /config - Update user configuration
pub async fn update_config(
    State(state): State<AppState>,
    Json(payload): Json<UpdateConfigRequest>,
) -> ApiResult<Json<UserConfig>> {
    // For MVP, we'll use a hardcoded user ID (alice)
    // In production, this would come from the authenticated session
    let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001")
        .expect("Invalid hardcoded UUID");

    let pool = state.db.pool.clone();
    let config_repo = ConfigRepository::new(pool);

    // Get current config
    let mut config = config_repo
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
    config_repo
        .update(&config)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(config))
}
