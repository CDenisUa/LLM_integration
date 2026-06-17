//! Listening-progress endpoints (per-book playback position).

// Core
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
// Services
use crate::db;
use crate::models::Progress;
use crate::state::AppState;

type ApiError = (StatusCode, Json<serde_json::Value>);

#[derive(Deserialize)]
pub struct ProgressUpdate {
    #[serde(default)]
    pub chapter_id: Option<String>,
    #[serde(default)]
    pub position_seconds: f64,
    #[serde(default)]
    pub total_listened: f64,
}

pub async fn set_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(update): Json<ProgressUpdate>,
) -> Result<StatusCode, ApiError> {
    db::upsert_progress(
        &state.db,
        &id,
        update.chapter_id.as_deref(),
        update.position_seconds,
        update.total_listened,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Option<Progress>>, ApiError> {
    db::get_progress(&state.db, &id).await.map(Json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })
}
