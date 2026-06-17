//! Shared application state passed to axum handlers.

// Services
use crate::db::Db;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
}
