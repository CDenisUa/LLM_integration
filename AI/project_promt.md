Personal AI Audiobook Generator

Project Goal

Build a personal AI-powered audiobook generator.

The application is made only for private personal use. The main purpose is to help the owner read more books by converting text books into audio files that can be listened to later.

The app should allow the user to upload a book, extract and clean the text, split it into chapters and chunks, generate speech using local AI text-to-speech engines, and listen to the result in a built-in audio player.

This is not a public audiobook marketplace, not a piracy tool, and not a service for distributing copyrighted books. All generated audio is private and available only to the local user.

⸻

Core Idea

The user uploads a book file.

Supported initial formats:

* .txt
* .epub
* .fb2
* later: .pdf
* later: .docx

The application extracts the text, filters and normalizes it, splits it into logical parts, and generates audio per chapter.

The generated audio should be saved locally and connected to the book inside the application library.

⸻

Main User Flow

1. User opens the app.
2. User uploads a book file.
3. App extracts text from the file.
4. App shows basic book info:
    * title
    * author if available
    * number of chapters
    * estimated reading time
    * estimated audio duration
5. App cleans and normalizes the extracted text.
6. User can preview the cleaned text.
7. User selects TTS engine and voice.
8. User starts generation.
9. App generates audio chunk by chunk.
10. App shows progress:

* current chapter
* current chunk
* percentage
* generated duration
* errors if any

11. After generation, user can listen to the audiobook in the app.
12. App remembers playback position for each book.

⸻

Main Features

1. Book Upload

The app should allow uploading local book files.

Initial support:

* TXT
* EPUB
* FB2

Later support:

* PDF
* DOCX
* MOBI / AZW3 through Calibre integration

The uploaded file should be stored locally in the app data directory.

The app should create a book record in the local database.

Each book should have:

* id
* title
* author
* original file path
* extracted text path
* cleaned text path
* cover image if available
* status
* created date
* updated date

Book statuses:

* uploaded
* extracting_text
* text_extracted
* cleaning_text
* ready_for_generation
* generating_audio
* generated
* failed

⸻

2. Text Extraction

The app should extract text from book files.

For EPUB:

* extract metadata
* extract title
* extract author
* extract chapters
* preserve chapter order
* remove HTML tags
* preserve paragraphs

For TXT:

* detect encoding
* normalize line breaks
* preserve paragraphs

For FB2:

* parse XML
* extract title and author
* extract body sections
* preserve chapter structure

For PDF later:

* extract text if selectable
* if scanned PDF, mark as requiring OCR
* OCR is not required in MVP

⸻

3. Text Cleaning and Filtering

The app must clean the extracted text before sending it to TTS.

The cleaner should handle Russian and English text.

Important cleaning rules:

* remove repeated page numbers
* remove headers and footers
* remove excessive whitespace
* remove broken line breaks
* fix hyphenated word breaks:
    * краси-\nвый should become красивый
* remove HTML entities
* remove table of contents artifacts
* remove ISBN lines
* remove copyright notes from generated audio content if they appear in metadata sections
* remove URLs
* remove email addresses
* remove image captions if detected
* remove footnote markers where possible
* normalize quotes
* normalize punctuation
* preserve dialogue formatting
* preserve paragraph boundaries
* preserve chapter titles
* preserve Russian letters including ё

The cleaner should not destroy literary text.

Important: do not over-clean dialogue lines. In Russian books, dialogue often starts with a dash. These lines must remain readable.

Example:

- Привет, - сказал он.

should stay as dialogue, not be removed.

⸻

4. Text Normalization for TTS

Before sending text to TTS, normalize common abbreviations.

For Russian:

* т.е. → то есть
* т.к. → так как
* и т.д. → и так далее
* и т.п. → и тому подобное
* г. → context-dependent, but by default год or keep unchanged if uncertain
* ул. → улица
* стр. → страница
* № → номер

For English:

* e.g. → for example
* i.e. → that is
* Mr. → Mister
* Mrs. → Misses
* Dr. → Doctor

Numbers can be left as digits at first. Later we can add number-to-words conversion.

The app should keep both versions:

* cleaned text
* TTS-normalized text

⸻

5. Chapter Detection

The app should detect chapters automatically.

Possible chapter markers:

* Глава 1
* Глава I
* Часть 1
* Пролог
* Эпилог
* Chapter 1
* Part 1
* Prologue
* Epilogue

If no chapters are detected, the app should split the book into virtual chapters by size.

Example:

* one virtual chapter per 10,000-20,000 characters

Each chapter should have:

* id
* book id
* title
* order index
* original text
* cleaned text
* audio status
* audio path
* duration if generated

⸻

6. Text Chunking

The app should split each chapter into smaller chunks for TTS generation.

Chunk size should be configurable.

Default:

* 1,500-3,000 characters per chunk

Chunking rules:

* do not cut in the middle of a sentence
* prefer splitting by paragraphs
* if paragraph is too long, split by sentence
* keep chunk text natural for speech
* preserve order
* each chunk should be independently regeneratable

Each chunk should have:

* id
* book id
* chapter id
* order index
* text
* status
* audio path
* error message if failed

Chunk statuses:

* pending
* generating
* generated
* failed
* skipped

⸻

7. TTS Engines

The app should support local TTS engines.

Initial engines:

1. Piper TTS
2. XTTS v2

Preferred strategy:

* Piper for fast generation
* XTTS v2 for better quality Russian voice

The app should use a TTS adapter interface so engines can be replaced or added later.

Example interface:

interface TtsEngine {
  id: string;
  name: string;
  supportedLanguages: string[];
  listVoices(): Promise<TtsVoice[]>;
  generateSpeech(input: GenerateSpeechInput): Promise<GenerateSpeechResult>;
}

The app should allow selecting:

* engine
* language
* voice
* speed
* output format

Initial output format:

* .wav for intermediate chunks
* .mp3 for final chapter files

Later:

* .m4b export with chapters

⸻

8. Russian Voice Support

The app must support Russian book narration.

Russian support is a core requirement.

Minimum requirement:

* at least one working Russian voice
* ability to generate Russian text into audio
* ability to process long Russian books

Preferred:

* multiple Russian voices
* male and female voices
* speed control
* stable pronunciation
* support for long-form narration

The app should allow testing a voice before generating the whole book.

Voice test text:

Это тест русской озвучки. Если голос звучит хорошо, можно начинать генерацию книги.

⸻

9. Audio Generation Pipeline

Audio generation should work as a background job.

The app should not freeze while generating audio.

Generation flow:

1. Take next pending chunk.
2. Send chunk text to selected TTS engine.
3. Save generated audio file.
4. Mark chunk as generated.
5. Continue with next chunk.
6. When all chunks in a chapter are generated, merge them into one chapter audio file.
7. When all chapters are generated, mark book as generated.

The app should support:

* pause generation
* resume generation
* retry failed chunk
* regenerate chapter
* regenerate full book
* delete generated audio

If generation fails, the app should save the error and continue safely.

⸻

10. Audio File Structure

Use a clean local file structure.

Example:

data/
  books/
    book-id/
      original/
        book.epub
      text/
        extracted.txt
        cleaned.txt
        normalized.txt
      chapters/
        chapter-001/
          chapter.txt
          chunks/
            chunk-001.txt
            chunk-001.wav
            chunk-002.txt
            chunk-002.wav
          chapter-001.mp3
        chapter-002/
          ...
      cover/
        cover.jpg
      exports/
        full-book.mp3
        full-book.m4b

⸻

11. Audio Player

The app should include a simple audiobook player.

Player features:

* play / pause
* next chapter
* previous chapter
* seek forward 10 seconds
* seek backward 10 seconds
* playback speed:
    * 0.75x
    * 1x
    * 1.25x
    * 1.5x
    * 2x
* chapter list
* current chapter progress
* full book progress
* remember last position
* continue listening button

The app should save listening progress locally.

For each book:

* current chapter
* current time in chapter
* total listened time
* last opened date

⸻

12. Book Library

The app should have a personal library screen.

Each book card should show:

* cover if available
* title
* author
* status
* generation progress
* listening progress
* duration if generated
* last listened date

Actions:

* open book
* generate audio
* continue listening
* delete book
* delete generated audio only
* view cleaned text
* edit metadata

⸻

13. Text Preview and Editor

The app should allow previewing the cleaned text before generation.

Basic editor features:

* view extracted text
* view cleaned text
* view normalized TTS text
* manually edit chapter text
* save edited text
* regenerate audio after edits

This is important because book parsing and cleaning will not always be perfect.

⸻

14. Local-First Design

The application should be local-first.

No cloud is required for MVP.

All books, text and audio files should be stored locally.

No user accounts are required.

No external database is required.

Recommended local database:

* SQLite

Recommended storage:

* local filesystem

⸻

15. Suggested Tech Stack

Frontend:

* React or Next.js
* TypeScript
* Tailwind CSS
* Zustand or Redux Toolkit for state
* HTML5 audio API

Backend:

* Python
* FastAPI
* SQLite
* SQLAlchemy
* background worker

TTS:

* Piper TTS
* XTTS v2

Audio tools:

* FFmpeg for audio conversion and merging

Book parsing:

* ebooklib for EPUB
* BeautifulSoup for HTML cleaning
* lxml for FB2
* charset-normalizer for TXT encoding
* later: pymupdf or pdfplumber for PDF

Development:

* Docker optional
* local development first

⸻

16. API Design

Possible backend endpoints:

POST   /api/books/upload
GET    /api/books
GET    /api/books/{bookId}
DELETE /api/books/{bookId}
POST   /api/books/{bookId}/extract
POST   /api/books/{bookId}/clean
POST   /api/books/{bookId}/generate
POST   /api/books/{bookId}/pause-generation
POST   /api/books/{bookId}/resume-generation
GET    /api/books/{bookId}/chapters
GET    /api/chapters/{chapterId}
PATCH  /api/chapters/{chapterId}
GET    /api/tts/engines
GET    /api/tts/voices
POST   /api/tts/test-voice
GET    /api/audio/{bookId}/{chapterId}
POST   /api/progress/{bookId}
GET    /api/progress/{bookId}

⸻

17. UI Screens

Required screens:

1. Library screen
2. Upload book screen
3. Book details screen
4. Text preview screen
5. Generation progress screen
6. Audio player screen
7. Settings screen

⸻

18. UI Style

The app should be clean, calm and focused on reading.

Visual direction:

* dark mode first
* minimal UI
* no visual noise
* comfortable typography
* clear progress indicators
* audiobook-like feeling
* private library feeling

The interface should not look like a public SaaS dashboard. It should feel like a personal reading tool.

⸻

19. Settings

Settings should include:

* default TTS engine
* default language
* default Russian voice
* default chunk size
* default audio format
* default playback speed
* local data folder
* delete generated audio
* clear cache

⸻

20. MVP Scope

The first working version should include only the most important features.

MVP must include:

* upload TXT
* upload EPUB
* extract text
* clean text
* split into chapters
* split chapters into chunks
* generate Russian audio with at least one engine
* save chapter audio files
* play generated audio
* remember playback position
* show generation progress

MVP should not include yet:

* public accounts
* payments
* cloud sync
* mobile apps
* OCR
* advanced voice cloning
* multi-user mode
* social sharing
* public audiobook publishing

⸻

21. Important Constraints

This app is for personal use only.

Do not build features for sharing copyrighted generated audiobooks.

Do not upload user books to third-party services unless explicitly configured.

Do not make cloud usage required.

Prefer local generation.

Prefer simple, stable architecture over complex microservices.

The app should work on a MacBook Pro with Apple Silicon.

⸻

22. Future Improvements

After MVP, add:

* PDF support
* OCR for scanned books
* M4B export with chapters
* better Russian text normalization
* voice presets
* automatic cover extraction
* sleep timer
* bookmarks
* notes
* offline PWA player
* mobile-friendly UI
* batch generation queue
* multi-voice narration for dialogues
* automatic dialogue detection
* per-character voices
* cloud GPU optional mode
* local desktop app using Tauri

⸻

Final Product Vision

The final product should be a private personal AI audiobook studio.

The user should be able to take a book, convert it into a clean audiobook, and listen to it comfortably.

The product should help the user read more books, learn more, and reduce friction between owning a book and actually consuming it.

The main success metric is simple:

The user uploads a book and can listen to it later as a clean, understandable audiobook.