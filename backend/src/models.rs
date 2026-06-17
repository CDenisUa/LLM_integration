//! Database row models for the library (books, chapters, chunks, progress).

// Core
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub original_path: Option<String>,
    pub cover_path: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Chapter {
    pub id: String,
    pub book_id: String,
    pub title: Option<String>,
    pub order_index: i64,
    pub text: String,
    pub status: String,
    pub audio_path: Option<String>,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Chunk {
    pub id: String,
    pub book_id: String,
    pub chapter_id: String,
    pub order_index: i64,
    pub text: String,
    pub status: String,
    pub audio_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Progress {
    pub book_id: String,
    pub chapter_id: Option<String>,
    pub position_seconds: f64,
    pub total_listened: f64,
    pub last_opened_at: Option<i64>,
}

/// Book lifecycle statuses (spec §1).
pub mod book_status {
    pub const UPLOADED: &str = "uploaded";
    pub const TEXT_EXTRACTED: &str = "text_extracted";
    pub const READY: &str = "ready_for_generation";
    pub const GENERATING: &str = "generating_audio";
    pub const GENERATED: &str = "generated";
    pub const FAILED: &str = "failed";
}

/// Chunk statuses (spec §6).
#[allow(dead_code)] // full status set kept for clarity; some are SQL defaults
pub mod chunk_status {
    pub const PENDING: &str = "pending";
    pub const GENERATING: &str = "generating";
    pub const GENERATED: &str = "generated";
    pub const FAILED: &str = "failed";
    pub const SKIPPED: &str = "skipped";
}
