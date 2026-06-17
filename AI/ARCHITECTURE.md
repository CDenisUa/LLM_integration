# Architecture — Personal AI Audiobook Generator

> Living document. Keep in sync with the codebase as phases land. See [PLAN.md](./PLAN.md) for the phased TDD roadmap and status.

## High-level shape

The product is built **on top of the existing `LLM_integration` lab**, not as a rewrite.

```
┌──────────────────────────┐      HTTP      ┌──────────────────────────┐
│  Frontend (Next.js 16)   │ ─────────────▶ │  Backend (Rust / axum)   │
│  React 19, zustand, TS   │ ◀───────────── │  SQLite + filesystem     │
└──────────────────────────┘   JSON / SSE   └────────────┬─────────────┘
                                                          │ HTTP (local)
                                                          ▼
                                            ┌──────────────────────────┐
                                            │  TTS sidecar (Python)    │
                                            │  FastAPI + Coqui XTTS v2 │
                                            │  loads model once        │
                                            └──────────────────────────┘
                                                          │ subprocess
                                                          ▼
                                                   ffmpeg (merge/mp3)
```

### Why this split
- **Keep Rust backend.** The spec suggested Python/FastAPI, but the real codebase is Rust/axum and already half-built (PDF reader, chat, pipeline). Rewriting would throw away working code.
- **Python only where it must be.** XTTS v2 is a PyTorch model — it needs Python. We isolate it in a thin FastAPI sidecar that the Rust backend calls over `http://127.0.0.1:<TTS_LOCAL_PORT>`. The sidecar loads the model once and stays warm.
- **Pure functions in Rust** for extraction, cleaning, normalization, chapter detection, chunking → easily unit-tested (TDD).

## Components

### 1. TTS sidecar — `tts_service/`
- FastAPI app, runs in its own venv (Python 3.10/3.11 recommended; 3.9 works).
- Loads `tts_models/multilingual/multi-dataset/xtts_v2` via the maintained `coqui-tts` fork.
- Endpoints:
  - `GET  /health` → `{status, model_loaded, device}`
  - `GET  /voices` → built-in studio speakers (+ any user reference wavs)
  - `POST /synthesize` `{text, language, speaker|speaker_wav, speed}` → WAV bytes (base64 in JSON, shape compatible with existing frontend)
- License: XTTS v2 under Coqui Public Model License (CPML), non-commercial. App is personal-use only — compliant.

### 2. Rust backend — `backend/`
- **Storage:** SQLite at `storage/library.db` (via `sqlx`), book assets on filesystem under `storage/books/<book-id>/...` (layout per spec §10).
- **TTS adapter:** `TtsEngine` trait with a `local` provider (calls sidecar) + existing cloud providers kept as fallback/optional.
- **Domain modules (new):** `db`, `models`, `extract`, `clean`, `normalize`, `chapters`, `chunking`, `tts`, `audio` (ffmpeg), `jobs` (background generation worker).
- **Routes:** books / chapters / tts / progress per spec §16.

### 3. Frontend — `frontend/`
- New screens: Library, Upload, Book details, Text preview, Generation progress, Audio player, Settings (spec §17).
- zustand stores; HTML5 audio player with persisted playback position.
- Reuses `AppShell` / `Sidebar`.

## Data model (SQLite)

- **books**: id, title, author, original_path, status, cover_path, created_at, updated_at
- **chapters**: id, book_id, title, order_index, audio_path, duration, status
- **chunks**: id, book_id, chapter_id, order_index, text, status, audio_path, error
- **progress**: book_id, chapter_id, position_seconds, total_listened, last_opened_at
- **settings**: key/value (engine, language, voice, chunk_size, format, speed)

Status enums per spec §1 (book), §6 (chunk).

## Filesystem layout (per book)
```
storage/books/<book-id>/
  original/      cleaned/normalized text/
  chapters/chapter-NNN/chunks/chunk-NNN.{txt,wav}
  chapters/chapter-NNN/chapter-NNN.mp3
  cover/  exports/
```

## Testing strategy (TDD)
- **Rust unit tests** (`#[cfg(test)]`) for every pure function — written *before* implementation.
- **Rust integration tests** for routes + DB against a temp SQLite (reuse `TestDirGuard` pattern).
- **TTS adapter** tested against a mock HTTP server (no model needed in CI).
- **Sidecar** has a contract test with a stubbed engine; real-model synthesis is a manual/marked test.
- **Frontend** uses vitest for stores/utilities.
