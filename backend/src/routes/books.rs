//! Book endpoints: upload, list/get/delete, and the extract → clean pipeline.

// Core
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::Json;
use std::path::Path as StdPath;
use tokio::fs;
use uuid::Uuid;
// Services
use crate::chapters::chapterize;
use crate::chunking::{chunk_text, DEFAULT_MAX_CHUNK_CHARS};
use crate::clean::clean_text;
use crate::db;
use crate::extract::{self, ExtractedBook};
use crate::models::{book_status, Book, Chapter};
use crate::normalize::normalize_for_tts;
use crate::state::AppState;

type ApiError = (StatusCode, Json<serde_json::Value>);

fn err(status: StatusCode, message: impl Into<String>) -> ApiError {
    (status, Json(serde_json::json!({ "error": message.into() })))
}

fn book_dir(id: &str) -> String {
    format!("storage/books/{id}")
}

/// Dispatch extraction by file extension.
pub fn extract_by_ext(ext: &str, bytes: &[u8]) -> Result<ExtractedBook, String> {
    match ext.to_lowercase().as_str() {
        "txt" => Ok(extract::txt::extract_txt(bytes)),
        "epub" => extract::epub::extract_epub(bytes),
        "fb2" => extract::fb2::extract_fb2(bytes),
        other => Err(format!("unsupported format: .{other}")),
    }
}

fn ext_of(path: &str) -> Option<String> {
    StdPath::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

/// Read the original file, extract text, store chapters, advance book status.
/// Returns the number of chapters created.
pub async fn run_extract(db: &db::Db, book_id: &str, path: &str, ext: &str) -> Result<usize, String> {
    let bytes = fs::read(path).await.map_err(|e| format!("read failed: {e}"))?;
    let extracted = extract_by_ext(ext, &bytes)?;

    if let Some(title) = extracted.title.as_deref() {
        db::update_book_meta(db, book_id, title, extracted.author.as_deref())
            .await
            .map_err(|e| e.to_string())?;
    }

    let chapters = chapterize(&extracted.sections);
    for (index, chapter) in chapters.iter().enumerate() {
        db::insert_chapter(
            db,
            &Uuid::new_v4().to_string(),
            book_id,
            chapter.title.as_deref(),
            index as i64,
            &chapter.text,
        )
        .await
        .map_err(|e| e.to_string())?;
    }

    db::update_book_status(db, book_id, book_status::TEXT_EXTRACTED)
        .await
        .map_err(|e| e.to_string())?;
    Ok(chapters.len())
}

/// Clean + normalize each chapter, (re)build TTS chunks, advance status.
/// Returns the number of chunks created.
pub async fn run_clean(db: &db::Db, book_id: &str, max_chunk_chars: usize) -> Result<usize, String> {
    let chapters = db::list_chapters(db, book_id)
        .await
        .map_err(|e| e.to_string())?;

    db::delete_chunks_for_book(db, book_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut order: i64 = 0;
    for chapter in &chapters {
        let cleaned = clean_text(&chapter.text);
        db::update_chapter_text(db, &chapter.id, &cleaned)
            .await
            .map_err(|e| e.to_string())?;

        let normalized = normalize_for_tts(&cleaned);
        for piece in chunk_text(&normalized, max_chunk_chars) {
            db::insert_chunk(
                db,
                &Uuid::new_v4().to_string(),
                book_id,
                &chapter.id,
                order,
                &piece,
            )
            .await
            .map_err(|e| e.to_string())?;
            order += 1;
        }
    }

    db::update_book_status(db, book_id, book_status::READY)
        .await
        .map_err(|e| e.to_string())?;
    Ok(order as usize)
}

// ---- HTTP handlers --------------------------------------------------------

pub async fn upload_book(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Book>, ApiError> {
    let mut filename = "untitled.txt".to_string();
    let mut file_data: Vec<u8> = Vec::new();
    let mut cover_data: Vec<u8> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                if let Some(name) = field.file_name() {
                    filename = name.to_string();
                }
                if let Ok(bytes) = field.bytes().await {
                    file_data.extend_from_slice(&bytes);
                }
            }
            Some("cover") => {
                if let Ok(bytes) = field.bytes().await {
                    cover_data.extend_from_slice(&bytes);
                }
            }
            _ => {}
        }
    }

    if file_data.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "no file uploaded"));
    }

    let id = Uuid::new_v4().to_string();
    let dir = book_dir(&id);
    fs::create_dir_all(format!("{dir}/original"))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let original_path = format!("{dir}/original/{filename}");
    fs::write(&original_path, &file_data)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut cover_path: Option<String> = None;
    if !cover_data.is_empty() {
        fs::create_dir_all(format!("{dir}/cover")).await.ok();
        let path = format!("{dir}/cover/cover.jpg");
        if fs::write(&path, &cover_data).await.is_ok() {
            cover_path = Some(path);
        }
    }

    let title = StdPath::new(&filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&filename)
        .to_string();

    let book = db::create_book(
        &state.db,
        &id,
        &title,
        None,
        Some(&original_path),
        cover_path.as_deref(),
        book_status::UPLOADED,
    )
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(book))
}

pub async fn list_books(State(state): State<AppState>) -> Result<Json<Vec<Book>>, ApiError> {
    db::list_books(&state.db)
        .await
        .map(Json)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn get_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Book>, ApiError> {
    match db::get_book(&state.db, &id).await {
        Ok(Some(book)) => Ok(Json(book)),
        Ok(None) => Err(err(StatusCode::NOT_FOUND, "book not found")),
        Err(e) => Err(err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn delete_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    db::delete_book(&state.db, &id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let _ = fs::remove_dir_all(book_dir(&id)).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn extract_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let book = db::get_book(&state.db, &id)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "book not found"))?;

    let path = book
        .original_path
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "book has no original file"))?;
    let ext = ext_of(&path).ok_or_else(|| err(StatusCode::BAD_REQUEST, "unknown file type"))?;

    match run_extract(&state.db, &id, &path, &ext).await {
        Ok(count) => Ok(Json(serde_json::json!({ "chapters": count }))),
        Err(e) => {
            let _ = db::update_book_status(&state.db, &id, book_status::FAILED).await;
            Err(err(StatusCode::UNPROCESSABLE_ENTITY, e))
        }
    }
}

pub async fn clean_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match run_clean(&state.db, &id, DEFAULT_MAX_CHUNK_CHARS).await {
        Ok(count) => Ok(Json(serde_json::json!({ "chunks": count }))),
        Err(e) => Err(err(StatusCode::UNPROCESSABLE_ENTITY, e)),
    }
}

pub async fn list_chapters(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Chapter>>, ApiError> {
    db::list_chapters(&state.db, &id)
        .await
        .map(Json)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(contents: &[u8], ext: &str) -> String {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("book-{unique}.{ext}"));
        std::fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn extract_by_ext_rejects_unknown() {
        assert!(extract_by_ext("pdf", b"x").is_err());
        assert!(extract_by_ext("txt", b"hello").is_ok());
    }

    #[tokio::test]
    async fn extract_then_clean_builds_chapters_and_chunks() {
        let db = db::connect_memory().await;
        db::create_book(&db, "b1", "tmp", None, None, None, book_status::UPLOADED)
            .await
            .unwrap();

        let text = "Глава 1\n\nПервый абзац. Второй абзац.\n\nГлава 2\n\nТретий абзац.";
        let path = temp_file(text.as_bytes(), "txt");

        let chapters = run_extract(&db, "b1", &path, "txt").await.unwrap();
        assert_eq!(chapters, 2);
        assert_eq!(
            db::get_book(&db, "b1").await.unwrap().unwrap().status,
            book_status::TEXT_EXTRACTED
        );

        let chunks = run_clean(&db, "b1", 2500).await.unwrap();
        assert!(chunks >= 2);
        assert_eq!(
            db::get_book(&db, "b1").await.unwrap().unwrap().status,
            book_status::READY
        );
        // chunks are globally ordered and pending
        let stored = db::list_chunks_for_book(&db, "b1").await.unwrap();
        assert_eq!(stored.len(), chunks);
        assert_eq!(stored[0].order_index, 0);
        assert!(stored.iter().all(|c| c.status == "pending"));

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn clean_is_idempotent_and_rebuilds_chunks() {
        let db = db::connect_memory().await;
        db::create_book(&db, "b1", "tmp", None, None, None, book_status::UPLOADED)
            .await
            .unwrap();
        let path = temp_file("Просто текст книги.".as_bytes(), "txt");
        run_extract(&db, "b1", &path, "txt").await.unwrap();

        let first = run_clean(&db, "b1", 2500).await.unwrap();
        let second = run_clean(&db, "b1", 2500).await.unwrap();
        assert_eq!(first, second);
        assert_eq!(db::list_chunks_for_book(&db, "b1").await.unwrap().len(), second);

        std::fs::remove_file(&path).ok();
    }
}
