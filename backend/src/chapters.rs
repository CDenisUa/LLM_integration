//! Chapter detection. Splits a book into chapters by heading markers
//! (Глава/Часть/Пролог/Эпилог/Chapter/Part/Prologue/Epilogue), falling back to
//! size-based virtual chapters when no markers are found.

// Core
use std::sync::OnceLock;
use regex::Regex;
// Services
use crate::extract::{normalize_whitespace, Section};

/// One detected chapter of a book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub title: Option<String>,
    pub text: String,
}

/// Default size (in characters) for a virtual chapter when no markers exist.
pub const DEFAULT_VIRTUAL_CHAPTER_CHARS: usize = 15_000;

/// Return the heading text if `line` looks like a chapter marker, else `None`.
fn heading_title(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 80 {
        return None;
    }

    static NUMBERED: OnceLock<Regex> = OnceLock::new();
    let numbered = NUMBERED.get_or_init(|| {
        Regex::new(r"(?i)^(глава|часть|chapter|part)\s+(\d+|[ivxlcdm]+)\b").unwrap()
    });
    static STANDALONE: OnceLock<Regex> = OnceLock::new();
    let standalone = STANDALONE
        .get_or_init(|| Regex::new(r"(?i)^(пролог|эпилог|prologue|epilogue)$").unwrap());

    if numbered.is_match(trimmed) || standalone.is_match(trimmed) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

/// Detect chapters in a single text blob via heading markers; if none are found,
/// fall back to [`split_into_virtual_chapters`].
pub fn detect_chapters(text: &str) -> Vec<Chapter> {
    let mut chapters = Vec::new();
    let mut cur_title: Option<String> = None;
    let mut cur_text = String::new();
    let mut seen_heading = false;

    for line in text.lines() {
        if let Some(title) = heading_title(line) {
            if seen_heading || !cur_text.trim().is_empty() {
                chapters.push(Chapter {
                    title: cur_title.take(),
                    text: normalize_whitespace(&cur_text),
                });
            }
            cur_title = Some(title);
            cur_text.clear();
            seen_heading = true;
        } else {
            cur_text.push_str(line);
            cur_text.push('\n');
        }
    }

    if cur_title.is_some() || !cur_text.trim().is_empty() {
        chapters.push(Chapter {
            title: cur_title,
            text: normalize_whitespace(&cur_text),
        });
    }

    if !seen_heading {
        return split_into_virtual_chapters(text, DEFAULT_VIRTUAL_CHAPTER_CHARS);
    }
    chapters
}

/// Split text into virtual chapters of roughly `target_chars`, breaking only on
/// paragraph boundaries.
pub fn split_into_virtual_chapters(text: &str, target_chars: usize) -> Vec<Chapter> {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    let mut chapters = Vec::new();
    let mut buf = String::new();
    for para in paragraphs {
        if !buf.is_empty() && buf.chars().count() + para.chars().count() > target_chars {
            chapters.push(Chapter {
                title: None,
                text: buf.trim().to_string(),
            });
            buf.clear();
        }
        if !buf.is_empty() {
            buf.push_str("\n\n");
        }
        buf.push_str(para);
    }
    if !buf.trim().is_empty() {
        chapters.push(Chapter {
            title: None,
            text: buf.trim().to_string(),
        });
    }
    if chapters.is_empty() {
        chapters.push(Chapter {
            title: None,
            text: text.trim().to_string(),
        });
    }
    chapters
}

/// Turn extracted sections into chapters. Multi-section sources (epub/fb2) map
/// one chapter per section; a single section is run through marker detection.
pub fn chapterize(sections: &[Section]) -> Vec<Chapter> {
    if sections.len() > 1 {
        return sections
            .iter()
            .map(|s| Chapter {
                title: s.title.clone(),
                text: s.text.clone(),
            })
            .collect();
    }
    match sections.first() {
        Some(section) => detect_chapters(&section.text),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_numbered_and_standalone_headings() {
        let text = "Пролог\n\nВступление.\n\nГлава 1\n\nПервая глава.\n\nГлава 2\n\nВторая глава.";
        let chapters = detect_chapters(text);
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title.as_deref(), Some("Пролог"));
        assert_eq!(chapters[0].text, "Вступление.");
        assert_eq!(chapters[1].title.as_deref(), Some("Глава 1"));
        assert_eq!(chapters[2].title.as_deref(), Some("Глава 2"));
    }

    #[test]
    fn keeps_leading_text_before_first_heading() {
        let text = "Предисловие автора.\n\nChapter 1\n\nBody.";
        let chapters = detect_chapters(text);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, None);
        assert_eq!(chapters[0].text, "Предисловие автора.");
        assert_eq!(chapters[1].title.as_deref(), Some("Chapter 1"));
    }

    #[test]
    fn does_not_treat_prose_sentence_as_heading() {
        // "Часть города была тиха." must not be detected as a "Часть" marker.
        let text = "Часть города была тиха и пуста.\n\nЛюди спали.";
        let chapters = detect_chapters(text);
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, None);
    }

    #[test]
    fn falls_back_to_virtual_chapters_without_markers() {
        let para = "Длинный абзац текста книги.";
        let text = format!("{para}\n\n{para}\n\n{para}\n\n{para}");
        // target small enough to force multiple virtual chapters
        let chapters = split_into_virtual_chapters(&text, 40);
        assert!(chapters.len() >= 2);
        assert!(chapters.iter().all(|c| c.title.is_none()));
    }

    #[test]
    fn chapterize_maps_sections_one_to_one() {
        let sections = vec![
            Section { title: Some("A".into()), text: "x".into() },
            Section { title: Some("B".into()), text: "y".into() },
        ];
        let chapters = chapterize(&sections);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[1].title.as_deref(), Some("B"));
    }
}
