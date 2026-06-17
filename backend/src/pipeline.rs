//! Background audiobook generation pipeline.
//!
//! Pulls pending chunks, synthesizes each to a WAV, merges a chapter's chunks
//! into an MP3 once they are all done, and advances book/chapter status. A
//! shared cancel flag lets a run be paused; resuming simply runs it again
//! (pending chunks are picked up where they were left).

// Core
use std::sync::atomic::{AtomicBool, Ordering};
use serde::Serialize;
use tokio::fs;
// Services
use crate::audio::{ffmpeg_available, merge_wavs_to_mp3};
use crate::db::{self, Db};
use crate::models::{book_status, chunk_status, Chunk};
use crate::tts_client::SpeechSynthesizer;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct GenerationOutcome {
    pub generated: usize,
    pub failed: usize,
}

/// Aggregate generation progress for a book.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ProgressSummary {
    pub total: usize,
    pub generated: usize,
    pub failed: usize,
    pub pending: usize,
    pub percent: u8,
}

/// Compute progress from a book's chunks (pure).
pub fn summarize(chunks: &[Chunk]) -> ProgressSummary {
    let total = chunks.len();
    let generated = chunks.iter().filter(|c| c.status == chunk_status::GENERATED).count();
    let failed = chunks.iter().filter(|c| c.status == chunk_status::FAILED).count();
    let pending = total.saturating_sub(generated).saturating_sub(failed);
    let percent = if total == 0 {
        0
    } else {
        ((generated as f64 / total as f64) * 100.0).round() as u8
    };
    ProgressSummary { total, generated, failed, pending, percent }
}

fn chunk_dir(base_dir: &str, book_id: &str, chapter_id: &str) -> String {
    format!("{base_dir}/{book_id}/chapters/{chapter_id}/chunks")
}

/// Generate audio for all pending chunks of a book (until done or cancelled).
pub async fn generate_book<S: SpeechSynthesizer>(
    db: &Db,
    synth: &S,
    book_id: &str,
    base_dir: &str,
    cancel: &AtomicBool,
) -> Result<GenerationOutcome, String> {
    db::update_book_status(db, book_id, book_status::GENERATING)
        .await
        .map_err(|e| e.to_string())?;

    let mut outcome = GenerationOutcome::default();

    loop {
        if cancel.load(Ordering::Acquire) {
            break;
        }
        let Some(chunk) = db::next_pending_chunk(db, book_id).await.map_err(|e| e.to_string())?
        else {
            break;
        };

        db::update_chunk_status(db, &chunk.id, chunk_status::GENERATING, None, None)
            .await
            .map_err(|e| e.to_string())?;

        match synth.synthesize(&chunk.text).await {
            Ok(bytes) => {
                let dir = chunk_dir(base_dir, book_id, &chunk.chapter_id);
                fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
                let path = format!("{dir}/{}.wav", chunk.id);
                fs::write(&path, &bytes).await.map_err(|e| e.to_string())?;
                db::update_chunk_status(db, &chunk.id, chunk_status::GENERATED, Some(&path), None)
                    .await
                    .map_err(|e| e.to_string())?;
                outcome.generated += 1;
            }
            Err(e) => {
                db::update_chunk_status(db, &chunk.id, chunk_status::FAILED, None, Some(&e))
                    .await
                    .map_err(|e| e.to_string())?;
                outcome.failed += 1;
            }
        }
    }

    merge_completed_chapters(db, book_id, base_dir).await;
    finalize_book_status(db, book_id).await?;
    Ok(outcome)
}

/// Merge every chapter whose chunks are all generated into a chapter MP3.
async fn merge_completed_chapters(db: &Db, book_id: &str, base_dir: &str) {
    let Ok(chapters) = db::list_chapters(db, book_id).await else {
        return;
    };
    if !ffmpeg_available() {
        return;
    }

    for chapter in chapters {
        if chapter.status == chunk_status::GENERATED {
            continue;
        }
        let Ok(chunks) = db::list_chunks_for_chapter(db, &chapter.id).await else {
            continue;
        };
        if chunks.is_empty()
            || !chunks
                .iter()
                .all(|c| c.status == chunk_status::GENERATED && c.audio_path.is_some())
        {
            continue;
        }
        let inputs: Vec<String> = chunks.iter().filter_map(|c| c.audio_path.clone()).collect();
        let output = format!("{base_dir}/{book_id}/chapters/{}/chapter.mp3", chapter.id);
        match merge_wavs_to_mp3(&inputs, &output) {
            Ok(()) => {
                let _ = db::set_chapter_audio(db, &chapter.id, &output, None, chunk_status::GENERATED).await;
            }
            Err(e) => tracing::warn!("chapter merge failed for {}: {e}", chapter.id),
        }
    }
}

async fn finalize_book_status(db: &Db, book_id: &str) -> Result<(), String> {
    let chunks = db::list_chunks_for_book(db, book_id).await.map_err(|e| e.to_string())?;
    let summary = summarize(&chunks);
    let status = if summary.total > 0 && summary.pending == 0 && summary.failed == 0 {
        book_status::GENERATED
    } else if summary.failed > 0 && summary.pending == 0 {
        book_status::FAILED
    } else {
        // pending remain (cancelled/paused) — keep it resumable
        book_status::GENERATING
    };
    db::update_book_status(db, book_id, status)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct FakeSynth {
        fail_on: Option<String>,
    }

    impl SpeechSynthesizer for FakeSynth {
        async fn synthesize(&self, text: &str) -> Result<Vec<u8>, String> {
            if self.fail_on.as_deref() == Some(text) {
                return Err("boom".to_string());
            }
            Ok(make_wav(1200))
        }
    }

    fn make_wav(frames: u32) -> Vec<u8> {
        let data_len = frames * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data_len).to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&24000u32.to_le_bytes());
        b.extend_from_slice(&48000u32.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data_len.to_le_bytes());
        b.extend(std::iter::repeat(0u8).take(data_len as usize));
        b
    }

    fn temp_dir() -> String {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("gen-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir.to_string_lossy().into_owned()
    }

    async fn seed(db: &Db, chunks: &[(&str, &str)]) {
        db::create_book(db, "b1", "T", None, None, None, book_status::READY).await.unwrap();
        db::insert_chapter(db, "ch1", "b1", Some("Глава 1"), 0, "").await.unwrap();
        for (i, (id, text)) in chunks.iter().enumerate() {
            db::insert_chunk(db, id, "b1", "ch1", i as i64, text).await.unwrap();
        }
    }

    #[test]
    fn summarize_computes_percent() {
        let mk = |status: &str| Chunk {
            id: "x".into(),
            book_id: "b".into(),
            chapter_id: "c".into(),
            order_index: 0,
            text: String::new(),
            status: status.into(),
            audio_path: None,
            error: None,
        };
        let chunks = vec![mk("generated"), mk("generated"), mk("pending"), mk("failed")];
        let s = summarize(&chunks);
        assert_eq!((s.total, s.generated, s.failed, s.pending), (4, 2, 1, 1));
        assert_eq!(s.percent, 50);
    }

    #[tokio::test]
    async fn generates_all_chunks_and_marks_book_generated() {
        let db = db::connect_memory().await;
        seed(&db, &[("k1", "Первый"), ("k2", "Второй")]).await;
        let base = temp_dir();
        let cancel = AtomicBool::new(false);

        let synth = FakeSynth { fail_on: None };
        let outcome = generate_book(&db, &synth, "b1", &base, &cancel).await.unwrap();

        assert_eq!(outcome.generated, 2);
        assert_eq!(outcome.failed, 0);
        assert!(std::path::Path::new(&format!("{base}/b1/chapters/ch1/chunks/k1.wav")).exists());
        assert_eq!(
            db::get_book(&db, "b1").await.unwrap().unwrap().status,
            book_status::GENERATED
        );
        std::fs::remove_dir_all(&base).ok();
    }

    #[tokio::test]
    async fn failed_chunk_does_not_stop_others_and_marks_book_failed() {
        let db = db::connect_memory().await;
        seed(&db, &[("k1", "ok"), ("k2", "bad"), ("k3", "ok2")]).await;
        let base = temp_dir();
        let cancel = AtomicBool::new(false);

        let synth = FakeSynth { fail_on: Some("bad".to_string()) };
        let outcome = generate_book(&db, &synth, "b1", &base, &cancel).await.unwrap();

        assert_eq!(outcome.generated, 2);
        assert_eq!(outcome.failed, 1);
        assert_eq!(
            db::get_book(&db, "b1").await.unwrap().unwrap().status,
            book_status::FAILED
        );
        std::fs::remove_dir_all(&base).ok();
    }

    #[tokio::test]
    async fn cancel_before_run_generates_nothing_and_stays_resumable() {
        let db = db::connect_memory().await;
        seed(&db, &[("k1", "a"), ("k2", "b")]).await;
        let base = temp_dir();
        let cancel = AtomicBool::new(true);

        let synth = FakeSynth { fail_on: None };
        let outcome = generate_book(&db, &synth, "b1", &base, &cancel).await.unwrap();

        assert_eq!(outcome.generated, 0);
        assert_eq!(
            db::get_book(&db, "b1").await.unwrap().unwrap().status,
            book_status::GENERATING
        );
        std::fs::remove_dir_all(&base).ok();
    }
}
