//! Generation control endpoints: start/pause/resume/retry/regenerate + progress.

// Core
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
// Services
use crate::db;
use crate::pipeline::{generate_book, summarize};
use crate::state::AppState;
use crate::tts_client::LocalSynthesizer;

type ApiError = (StatusCode, Json<serde_json::Value>);

const BOOKS_DIR: &str = "storage/books";

fn err(status: StatusCode, message: impl Into<String>) -> ApiError {
    (status, Json(serde_json::json!({ "error": message.into() })))
}

/// Spawn a background generation task for `book_id` and register its cancel flag.
fn spawn_generation(state: &AppState, book_id: String) {
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .jobs
        .lock()
        .unwrap()
        .insert(book_id.clone(), cancel.clone());

    let db = state.db.clone();
    let jobs = state.jobs.clone();
    tokio::spawn(async move {
        let synth = LocalSynthesizer::from_env();
        if let Err(e) = generate_book(&db, &synth, &book_id, BOOKS_DIR, &cancel).await {
            tracing::error!("generation failed for {book_id}: {e}");
        }
        jobs.lock().unwrap().remove(&book_id);
    });
}

fn is_running(state: &AppState, book_id: &str) -> bool {
    state.jobs.lock().unwrap().contains_key(book_id)
}

async fn start(state: &AppState, book_id: &str) -> Result<Json<serde_json::Value>, ApiError> {
    let chunks = db::list_chunks_for_book(&state.db, book_id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if chunks.is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "no chunks to generate — run /clean first",
        ));
    }
    if is_running(state, book_id) {
        return Ok(Json(serde_json::json!({ "status": "already_running" })));
    }
    spawn_generation(state, book_id.to_string());
    Ok(Json(serde_json::json!({ "status": "started" })))
}

pub async fn generate(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    start(&state, &id).await.map(|j| (StatusCode::ACCEPTED, j))
}

pub async fn resume(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    start(&state, &id).await.map(|j| (StatusCode::ACCEPTED, j))
}

pub async fn pause(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if let Some(flag) = state.jobs.lock().unwrap().get(&id) {
        flag.store(true, Ordering::Release);
        return Json(serde_json::json!({ "status": "pausing" }));
    }
    Json(serde_json::json!({ "status": "not_running" }))
}

pub async fn retry(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    db::reset_failed_chunks(&state.db, &id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    start(&state, &id).await.map(|j| (StatusCode::ACCEPTED, j))
}

pub async fn regenerate(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    db::reset_all_chunks(&state.db, &id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    start(&state, &id).await.map(|j| (StatusCode::ACCEPTED, j))
}

pub async fn generation_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let chunks = db::list_chunks_for_book(&state.db, &id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let summary = summarize(&chunks);
    let status = db::get_book(&state.db, &id)
        .await
        .ok()
        .flatten()
        .map(|b| b.status);
    Ok(Json(serde_json::json!({
        "summary": summary,
        "status": status,
        "running": is_running(&state, &id),
    })))
}
