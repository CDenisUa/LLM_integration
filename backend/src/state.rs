//! Shared application state passed to axum handlers.

// Core
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
// Services
use crate::db::Db;

/// Per-book cancel flags for in-flight generation jobs.
pub type JobRegistry = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub jobs: JobRegistry,
}

impl AppState {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
