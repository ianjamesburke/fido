use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use badgelib::{Badge, Color, Style};

use crate::state::AppState;

const LABEL: &str = "fido";

/// Render a shields-flat-style badge: `fido | N members`. Pure — no I/O.
pub fn render_badge_svg(member_count: i64) -> String {
    let value = if member_count == 1 {
        "1 member".to_string()
    } else {
        format!("{} members", member_count)
    };

    Badge::new()
        .label(LABEL)
        .value(&value)
        .label_color(Color::Hex("555".into()))
        .value_color(Color::Hex("4c1".into()))
        .style(Style::FlatSquare)
        .to_svg()
}

/// GET /badge/:owner/:repo.svg — public, no auth.
pub async fn community_badge(
    State(state): State<AppState>,
    Path((owner, repo_svg)): Path<(String, String)>,
) -> Response {
    let Some(repo) = repo_svg.strip_suffix(".svg") else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };

    let community = match state.repos.communities.get_by_owner_name(&owner, repo) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "no fido community for this repo").into_response()
        }
        Err(error) => {
            tracing::warn!(%owner, repo, %error, "Badge community lookup failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "lookup failed").into_response();
        }
    };

    let count = match state.repos.communities.member_count(&community.id) {
        Ok(count) => count,
        Err(error) => {
            tracing::warn!(community_id = %community.id, %error, "Badge member count failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "count failed").into_response();
        }
    };

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        render_badge_svg(count),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_svg_contains_count_and_label() {
        let svg = render_badge_svg(14);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("fido"));
        assert!(svg.contains("14 members"));
    }

    #[test]
    fn badge_svg_singular_member() {
        assert!(render_badge_svg(1).contains("1 member"));
        assert!(!render_badge_svg(1).contains("1 members"));
    }
}
