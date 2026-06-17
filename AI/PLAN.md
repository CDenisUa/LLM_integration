# Development Plan — Personal AI Audiobook Generator

> Living roadmap. Update the **Status** column as work lands. Approach: **TDD** — write failing tests first, implement until green, then move on. See [ARCHITECTURE.md](./ARCHITECTURE.md) and the spec in [project_promt.md](./project_promt.md).

## Decisions (locked)
- **TTS engine:** Coqui TTS + **XTTS v2** (best Russian, voice cloning, local on Apple Silicon, CPU).
- **Scope:** full audiobook generator per spec, built incrementally.
- **Backend stays Rust/axum** — Python only for the XTTS sidecar.
- **DB:** SQLite (`sqlx`) at `storage/library.db`.
- **Audio:** ffmpeg (already installed) for chunk→chapter merge and mp3.

## Legend
Status: ⬜ todo · 🟡 in progress · ✅ done

---

## Phase 0 — Foundations & knowledge base ✅
- [x] Write `AI/ARCHITECTURE.md` and `AI/PLAN.md` (this file).
- [x] Confirm `cargo test` baseline green (existing pdf delete test).
- [x] Extend `backend/.env.example` with `TTS_LOCAL_URL`, `XTTS_*`, `DATABASE_URL`.
- [~] Rust deps (`sqlx`, `regex`, `quick-xml`, `zip`, encoding, dev `wiremock`/`tempfile`) — added incrementally per phase to avoid premature build bloat.

## Phase 1 — TTS sidecar (Coqui XTTS v2) ✅
- [x] `tts_service/`: FastAPI app (`app.py`, `engine.py`, `run.py`), `requirements.txt`, `requirements-dev.txt`, `setup.sh` (venv + `--download`), `README.md`.
- [x] Endpoints `/health`, `/voices`, `/synthesize` (returns `audio/wav` bytes; Rust will base64 for the frontend).
- [x] **Tests:** 6 pytest contract tests with a stub engine (routes, validation 422/400, error 500, WAV roundtrip) — pass without loading torch/model.
- [ ] Russian voice smoke: synthesize spec test phrase (needs `./setup.sh --download`; manual).
- [ ] Wire into `start-dev.sh` (optional auto-start) — deferred to Phase 2 wiring.

## Phase 2 — Rust `local` TTS provider ✅
- [x] `synthesize_local` calling the sidecar; `resolve_provider` (default `local`, cloud kept as fallback).
- [x] **Tests:** 4 tests vs `wiremock` (base64 WAV + JSON body, sidecar 500 error, unreachable→502, provider resolution). All green.
- [x] Provider `local` wired into `/api/tts` with config-aware cache hashing.
- [x] New routes: `GET /api/tts/engines`, `GET /api/tts/voices`, `POST /api/tts/test-voice` (defaults to RU test phrase).
- [x] Optional sidecar auto-start in `start-dev.sh` (only if `tts_service/.venv` exists).

## Phase 3 — Text extraction ✅ (TDD)
- [x] `extract::txt` — chardetng encoding detect (UTF-8/CP1251) + `normalize_whitespace`.
- [x] `extract::html` — `html_to_text` (paragraph-preserving, script/style drop) + `first_heading`.
- [x] `extract::epub` — zip + container.xml → OPF → spine order, metadata, HTML strip.
- [x] `extract::fb2` — quick-xml streaming, title/author, nested body sections.
- [x] **Tests:** 21 Rust tests total green (incl. in-memory epub fixture, CP1251 decode, ё preserved).

## Phase 4 — Cleaning + normalization ✅ (TDD)
- [x] `clean` — entities, URLs/emails, footnote markers, page-number/ISBN lines, hyphenation fix, quote/punct normalize. **Preserves RU dialogue dashes, paragraphs, titles, ё.** (headers/footers heuristic deferred to future.)
- [x] `normalize` — RU/EN abbreviation expansion (т.е., т.к., и т.д., ул., стр., №, e.g., Mr.…); `г.` left ambiguous; numbers kept as digits.
- [x] **Tests:** 16 new unit tests incl. dialogue-preservation + ё regression; 33 Rust tests total green.

## Phase 5 — Chapter detection + chunking ✅ (TDD)
- [x] `chapters` — `detect_chapters` (Глава/Часть/Пролог/Эпилог/Chapter/Part…, numbered+standalone), `split_into_virtual_chapters` fallback, `chapterize` for sections; prose sentences not misdetected as headings.
- [x] `chunking` — `chunk_text` (paragraph-preferred, sentence-safe via `split_sentences`), ordered, oversized-sentence emitted whole; `DEFAULT_MAX_CHUNK_CHARS=2500`.
- [x] **Tests:** 11 new unit tests (marker variants, no-marker fallback, no mid-sentence cuts, punctuation variants); 44 Rust tests total green.

## Phase 6 — DB + models + migrations ✅ (TDD)
- [x] `sqlx` SQLite pool + `migrations/0001_init.sql` (books/chapters/chunks/progress/settings + indexes, FK cascade).
- [x] `models` (Book/Chapter/Chunk/Progress + status constants); `db` CRUD (books, chapters, chunks incl. `next_pending_chunk`, progress upsert).
- [x] `AppState { db }` wired through `routes::router()` → `Router<AppState>` and `main`.
- [x] **Tests:** 4 async tests vs shared in-memory SQLite (CRUD, FK cascade, pending-chunk flow, progress upsert); 48 Rust tests total green.

## Phase 7 — Upload → extract → clean endpoints ✅
- [x] `routes::books`: `POST /books/upload` (multipart file+cover), `GET /books`, `GET/DELETE /books/:id`.
- [x] `POST /books/:id/extract` (format dispatch, chapterize, status → text_extracted; failure → failed) and `POST /books/:id/clean` (clean+normalize, rebuild ordered chunks, status → ready_for_generation).
- [x] `GET /books/:id/chapters`; book asset layout `storage/books/<id>/{original,cover}`.
- [x] **Tests:** 3 tests over extract→clean flow (in-memory DB + temp files), idempotent re-clean, format dispatch; 51 Rust tests total green.

## Phase 8 — Generation pipeline (background worker) ✅
- [x] `tts_client::SpeechSynthesizer` trait + `LocalSynthesizer` (sidecar); `audio::merge_wavs_to_mp3` (ffmpeg).
- [x] `pipeline::generate_book` generic over synthesizer: pending chunk → wav → chapter merge → mp3 → book status; cancel flag for pause; failure continues safely.
- [x] Endpoints: `generate`/`pause-generation`/`resume-generation`/`retry`/`regenerate` + `GET /books/:id/generation` (progress summary); job registry in `AppState`. Listening progress `GET/POST /progress/:id`. `/api/audio` serves generated files.
- [x] **Tests:** 6 new (full generate→generated, failure-continues→failed, cancel→resumable, progress summarize, ffmpeg merge, empty-input); 57 Rust tests total green; clean build.

## Phase 9 — Frontend screens ⬜
- [ ] Library, Upload, Book details, Text preview/editor, Generation progress, Audio player, Settings.
- [ ] zustand stores; HTML5 player (speed, ±10s, chapter nav, persisted position, continue listening).
- [ ] Dark-first, minimal, personal-library feel (spec §18).
- [ ] vitest for stores; add Chepio footer credit strip (global rule).

## Phase 10 — Polish & future hooks ⬜
- [ ] Cover extraction, exports (mp3; later m4b), settings persistence.
- [ ] Finalize docs; note PDF/OCR/m4b as future (spec §22).

---

## Changelog
- _2026-06-17_ — Plan + architecture authored. Decisions locked: XTTS v2 sidecar, Rust kept, SQLite, full scope, TDD.
- _2026-06-17_ — Phase 0 done (baseline green, `.env.example` extended). Phase 1 done: XTTS v2 sidecar with 6 passing contract tests (no model load), `setup.sh`, README.
- _2026-06-17_ — Phase 2 done: Rust `local` provider + engines/voices/test-voice routes; 4 wiremock-backed tests (5 total Rust tests green); sidecar auto-start in start-dev.sh.
- _2026-06-17_ — Phase 3 done: `extract` module (txt/html/fb2/epub) with shared `normalize_whitespace`; 21 Rust tests green.
- _2026-06-17_ — Phase 4 done: `clean` + `normalize` modules; dialogue/ё preserved; 33 Rust tests green.
- _2026-06-17_ — Phase 5 done: `chapters` (marker detect + virtual split) and `chunking` (sentence-safe); 44 Rust tests green.
- _2026-06-17_ — Phase 6 done: sqlx SQLite + migrations + models + CRUD; `AppState` wired into router; 48 Rust tests green.
- _2026-06-17_ — Phase 7 done: `routes::books` upload/list/get/delete + extract→clean pipeline endpoints; 51 Rust tests green.
- _2026-06-17_ — Phase 8 done: background generation pipeline (synthesizer trait, ffmpeg merge, controls, progress, audio serving); 57 Rust tests green, clean build.
