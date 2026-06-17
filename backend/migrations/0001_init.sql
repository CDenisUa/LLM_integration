-- Library schema for the audiobook generator.

CREATE TABLE IF NOT EXISTS books (
    id            TEXT PRIMARY KEY,
    title         TEXT NOT NULL,
    author        TEXT,
    original_path TEXT,
    cover_path    TEXT,
    status        TEXT NOT NULL DEFAULT 'uploaded',
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS chapters (
    id          TEXT PRIMARY KEY,
    book_id     TEXT NOT NULL,
    title       TEXT,
    order_index INTEGER NOT NULL,
    text        TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'pending',
    audio_path  TEXT,
    duration    REAL,
    FOREIGN KEY (book_id) REFERENCES books (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS chunks (
    id          TEXT PRIMARY KEY,
    book_id     TEXT NOT NULL,
    chapter_id  TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    text        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',
    audio_path  TEXT,
    error       TEXT,
    FOREIGN KEY (chapter_id) REFERENCES chapters (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS progress (
    book_id          TEXT PRIMARY KEY,
    chapter_id       TEXT,
    position_seconds REAL NOT NULL DEFAULT 0,
    total_listened   REAL NOT NULL DEFAULT 0,
    last_opened_at   INTEGER
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chapters_book ON chapters (book_id, order_index);
CREATE INDEX IF NOT EXISTS idx_chunks_book ON chunks (book_id, order_index);
CREATE INDEX IF NOT EXISTS idx_chunks_chapter ON chunks (chapter_id, order_index);
