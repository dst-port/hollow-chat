use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::social::user_id_by_username;
use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BadgeDto {
    pub slug: String,
    pub label: String,
    pub description: String,
}

pub async fn catalog(
    State(state): State<AppState>,
    _session: AuthSession,
) -> Result<Json<Vec<BadgeDto>>, AppError> {
    let badges: Vec<BadgeDto> =
        sqlx::query_as("SELECT slug, label, description FROM badges ORDER BY sort_order")
            .fetch_all(&state.pool)
            .await?;

    Ok(Json(badges))
}

pub async fn for_user(
    State(state): State<AppState>,
    _session: AuthSession,
    Path(username): Path<String>,
) -> Result<Json<Vec<String>>, AppError> {
    let Ok(user_id) = user_id_by_username(&state.pool, &username).await else {
        return Ok(Json(Vec::new()));
    };

    let slugs: Vec<(String,)> = sqlx::query_as(
        "SELECT badges.slug FROM user_badges \
         JOIN badges ON badges.slug = user_badges.badge_slug \
         WHERE user_badges.user_id = $1 \
         ORDER BY badges.sort_order",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(slugs.into_iter().map(|(slug,)| slug).collect()))
}

pub async fn award(pool: &sqlx::PgPool, user_id: Uuid, slug: &str) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO user_badges (user_id, badge_slug) VALUES ($1, $2) \
         ON CONFLICT (user_id, badge_slug) DO NOTHING",
    )
    .bind(user_id)
    .bind(slug)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn revoke(pool: &sqlx::PgPool, user_id: Uuid, slug: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM user_badges WHERE user_id = $1 AND badge_slug = $2")
        .bind(user_id)
        .bind(slug)
        .execute(pool)
        .await?;

    Ok(())
}
