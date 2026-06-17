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

## Phase 4 — Cleaning + normalization ⬜ (TDD)
- [ ] `clean` — page numbers, headers/footers, hyphenation fix, HTML entities, URLs/emails, footnote markers, quote/punct normalize. **Preserve RU dialogue dashes, paragraphs, chapter titles, ё.**
- [ ] `normalize` — RU/EN abbreviation expansion (spec §4); keep cleaned + normalized versions.
- [ ] **Tests:** rich table-driven cases incl. dialogue-preservation regression.

## Phase 5 — Chapter detection + chunking ⬜ (TDD)
- [ ] `chapters` — marker detection (Глава/Часть/Пролог/Эпилог/Chapter/Part…), virtual split by size fallback.
- [ ] `chunking` — sentence-safe split, paragraph-preferred, 1.5–3k chars configurable, ordered, independently regeneratable.
- [ ] **Tests:** marker variants, no-marker fallback, no mid-sentence cuts.

## Phase 6 — DB + models + migrations ⬜ (TDD)
- [ ] `sqlx` pool + migrations; `models` for Book/Chapter/Chunk/Progress/Settings.
- [ ] CRUD + status transitions.
- [ ] **Tests:** against a temp SQLite per test.

## Phase 7 — Upload → extract → clean endpoints ⬜
- [ ] `POST /api/books/upload`, `GET /api/books`, `GET/DELETE /api/books/{id}`.
- [ ] `POST /api/books/{id}/extract`, `/clean`; status transitions; cover handling.
- [ ] **Tests:** integration over full upload→ready_for_generation flow.

## Phase 8 — Generation pipeline (background worker) ⬜
- [ ] `jobs` worker: pending chunk → local TTS → wav; chapter merge (ffmpeg) → mp3; book→generated.
- [ ] Controls: pause / resume / retry chunk / regenerate chapter|book / delete audio.
- [ ] Progress reporting (poll or SSE): chapter, chunk, %, duration, errors.
- [ ] **Tests:** pipeline orchestration with a fake `TtsEngine`; failure-continues-safely.

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
