//! SQLite access layer (connection, migrations, CRUD) built on `sqlx`.

// Core
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
// Types
use crate::models::{Book, Chapter, Chunk, Progress};

pub type Db = sqlx::SqlitePool;
type DynError = Box<dyn std::error::Error + Send + Sync>;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Connect to (and migrate) the SQLite database at `database_url`,
/// e.g. `sqlite://storage/library.db`. The file is created if missing.
pub async fn connect(database_url: &str) -> Result<Db, DynError> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// A shared in-memory database (single connection) — used by tests.
#[cfg(test)]
pub async fn connect_memory() -> Db {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

// ---- Books ----------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub async fn create_book(
    db: &Db,
    id: &str,
    title: &str,
    author: Option<&str>,
    original_path: Option<&str>,
    cover_path: Option<&str>,
    status: &str,
) -> Result<Book, sqlx::Error> {
    let ts = now();
    sqlx::query(
        "INSERT INTO books (id, title, author, original_path, cover_path, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(title)
    .bind(author)
    .bind(original_path)
    .bind(cover_path)
    .bind(status)
    .bind(ts)
    .bind(ts)
    .execute(db)
    .await?;
    get_book(db, id).await.map(|b| b.expect("just inserted"))
}

pub async fn get_book(db: &Db, id: &str) -> Result<Option<Book>, sqlx::Error> {
    sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn list_books(db: &Db) -> Result<Vec<Book>, sqlx::Error> {
    sqlx::query_as::<_, Book>("SELECT * FROM books ORDER BY created_at DESC")
        .fetch_all(db)
        .await
}

pub async fn update_book_status(db: &Db, id: &str, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE books SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(now())
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_book(db: &Db, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM books WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

// ---- Chapters -------------------------------------------------------------

pub async fn insert_chapter(
    db: &Db,
    id: &str,
    book_id: &str,
    title: Option<&str>,
    order_index: i64,
    text: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO chapters (id, book_id, title, order_index, text) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(book_id)
    .bind(title)
    .bind(order_index)
    .bind(text)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list_chapters(db: &Db, book_id: &str) -> Result<Vec<Chapter>, sqlx::Error> {
    sqlx::query_as::<_, Chapter>(
        "SELECT * FROM chapters WHERE book_id = ? ORDER BY order_index",
    )
    .bind(book_id)
    .fetch_all(db)
    .await
}

pub async fn set_chapter_audio(
    db: &Db,
    id: &str,
    audio_path: &str,
    duration: Option<f64>,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chapters SET audio_path = ?, duration = ?, status = ? WHERE id = ?")
        .bind(audio_path)
        .bind(duration)
        .bind(status)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

// ---- Chunks ---------------------------------------------------------------

pub async fn insert_chunk(
    db: &Db,
    id: &str,
    book_id: &str,
    chapter_id: &str,
    order_index: i64,
    text: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO chunks (id, book_id, chapter_id, order_index, text) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(book_id)
    .bind(chapter_id)
    .bind(order_index)
    .bind(text)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list_chunks_for_book(db: &Db, book_id: &str) -> Result<Vec<Chunk>, sqlx::Error> {
    sqlx::query_as::<_, Chunk>("SELECT * FROM chunks WHERE book_id = ? ORDER BY order_index")
        .bind(book_id)
        .fetch_all(db)
        .await
}

/// The next chunk awaiting generation for a book (lowest order, status pending).
pub async fn next_pending_chunk(db: &Db, book_id: &str) -> Result<Option<Chunk>, sqlx::Error> {
    sqlx::query_as::<_, Chunk>(
        "SELECT * FROM chunks WHERE book_id = ? AND status = 'pending' ORDER BY order_index LIMIT 1",
    )
    .bind(book_id)
    .fetch_optional(db)
    .await
}

pub async fn update_chunk_status(
    db: &Db,
    id: &str,
    status: &str,
    audio_path: Option<&str>,
    error: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chunks SET status = ?, audio_path = ?, error = ? WHERE id = ?")
        .bind(status)
        .bind(audio_path)
        .bind(error)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

// ---- Progress -------------------------------------------------------------

pub async fn upsert_progress(
    db: &Db,
    book_id: &str,
    chapter_id: Option<&str>,
    position_seconds: f64,
    total_listened: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO progress (book_id, chapter_id, position_seconds, total_listened, last_opened_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(book_id) DO UPDATE SET
             chapter_id = excluded.chapter_id,
             position_seconds = excluded.position_seconds,
             total_listened = excluded.total_listened,
             last_opened_at = excluded.last_opened_at",
    )
    .bind(book_id)
    .bind(chapter_id)
    .bind(position_seconds)
    .bind(total_listened)
    .bind(now())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_progress(db: &Db, book_id: &str) -> Result<Option<Progress>, sqlx::Error> {
    sqlx::query_as::<_, Progress>("SELECT * FROM progress WHERE book_id = ?")
        .bind(book_id)
        .fetch_optional(db)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{book_status, chunk_status};

    #[tokio::test]
    async fn book_create_get_list_status_delete() {
        let db = connect_memory().await;

        let book = create_book(&db, "b1", "Война и мир", Some("Толстой"), None, None, book_status::UPLOADED)
            .await
            .unwrap();
        assert_eq!(book.title, "Война и мир");
        assert_eq!(book.status, book_status::UPLOADED);

        assert_eq!(list_books(&db).await.unwrap().len(), 1);

        update_book_status(&db, "b1", book_status::READY).await.unwrap();
        assert_eq!(get_book(&db, "b1").await.unwrap().unwrap().status, book_status::READY);

        delete_book(&db, "b1").await.unwrap();
        assert!(get_book(&db, "b1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn deleting_book_cascades_to_chapters_and_chunks() {
        let db = connect_memory().await;
        create_book(&db, "b1", "T", None, None, None, book_status::UPLOADED).await.unwrap();
        insert_chapter(&db, "c1", "b1", Some("Глава 1"), 0, "text").await.unwrap();
        insert_chunk(&db, "k1", "b1", "c1", 0, "chunk text").await.unwrap();

        assert_eq!(list_chapters(&db, "b1").await.unwrap().len(), 1);
        assert_eq!(list_chunks_for_book(&db, "b1").await.unwrap().len(), 1);

        delete_book(&db, "b1").await.unwrap();
        assert!(list_chapters(&db, "b1").await.unwrap().is_empty());
        assert!(list_chunks_for_book(&db, "b1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn chunks_order_and_next_pending_flow() {
        let db = connect_memory().await;
        create_book(&db, "b1", "T", None, None, None, book_status::UPLOADED).await.unwrap();
        insert_chapter(&db, "c1", "b1", None, 0, "").await.unwrap();
        insert_chunk(&db, "k1", "b1", "c1", 0, "first").await.unwrap();
        insert_chunk(&db, "k2", "b1", "c1", 1, "second").await.unwrap();

        let next = next_pending_chunk(&db, "b1").await.unwrap().unwrap();
        assert_eq!(next.id, "k1");

        update_chunk_status(&db, "k1", chunk_status::GENERATED, Some("k1.wav"), None)
            .await
            .unwrap();
        let next = next_pending_chunk(&db, "b1").await.unwrap().unwrap();
        assert_eq!(next.id, "k2");

        update_chunk_status(&db, "k2", chunk_status::GENERATED, Some("k2.wav"), None)
            .await
            .unwrap();
        assert!(next_pending_chunk(&db, "b1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn progress_upsert_overwrites() {
        let db = connect_memory().await;
        create_book(&db, "b1", "T", None, None, None, book_status::UPLOADED).await.unwrap();

        upsert_progress(&db, "b1", Some("c1"), 12.5, 12.5).await.unwrap();
        upsert_progress(&db, "b1", Some("c2"), 30.0, 42.5).await.unwrap();

        let p = get_progress(&db, "b1").await.unwrap().unwrap();
        assert_eq!(p.chapter_id.as_deref(), Some("c2"));
        assert_eq!(p.position_seconds, 30.0);
        assert_eq!(p.total_listened, 42.5);
    }
}
