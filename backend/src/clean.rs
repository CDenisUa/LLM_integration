//! Cleaning of extracted literary text before TTS.
//!
//! Rules are conservative: they remove deterministic noise (page numbers, ISBN,
//! URLs, footnote markers, hyphenation breaks) without destroying prose. Russian
//! dialogue lines (starting with a dash), paragraph boundaries, chapter titles
//! and the letter `ё` are preserved.

// Core
use std::sync::OnceLock;
use regex::Regex;
// Services
use crate::extract::normalize_whitespace;

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("valid regex"))
}

/// Decode the handful of HTML entities that survive plain-text sources.
pub fn decode_entities(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

/// Remove `http(s)://…` and `www.…` URLs.
pub fn remove_urls(input: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    re(&RE, r"(?i)\b(?:https?://|www\.)\S+")
        .replace_all(input, "")
        .into_owned()
}

/// Remove email addresses.
pub fn remove_emails(input: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    re(&RE, r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}")
        .replace_all(input, "")
        .into_owned()
}

/// Remove bracketed footnote markers like `[1]` or `{12}`.
pub fn remove_footnote_markers(input: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    re(&RE, r"[\[\{]\d{1,3}[\]\}]")
        .replace_all(input, "")
        .into_owned()
}

/// Drop lines that are page-number or ISBN artifacts. Dialogue and prose lines
/// are kept (a line starting with a dash is never a page number).
pub fn remove_artifact_lines(input: &str) -> String {
    static PAGE: OnceLock<Regex> = OnceLock::new();
    let page = re(&PAGE, r"^\s*(?:[-—–]\s*)?\d{1,4}(?:\s*[-—–])?\s*$");
    input
        .lines()
        .filter(|line| {
            if page.is_match(line) {
                return false;
            }
            if line.to_uppercase().contains("ISBN") {
                return false;
            }
            true
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Join words split across a line break by a hyphen: `краси-\nвый` → `красивый`.
/// Handles ASCII hyphen and the soft hyphen (U+00AD).
pub fn fix_hyphenation(input: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    re(&RE, r"(\p{L})[\-\x{00AD}][ \t]*\n[ \t]*(\p{L})")
        .replace_all(input, "$1$2")
        .into_owned()
}

/// Normalize curly quotes to straight ASCII; guillemets `«»` are left intact.
pub fn normalize_quotes(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => '"',
            '\u{2018}' | '\u{2019}' | '\u{201A}' => '\'',
            other => other,
        })
        .collect()
}

/// Normalize the ellipsis character and collapse runs of spaces/tabs.
pub fn normalize_punctuation(input: &str) -> String {
    static SPACES: OnceLock<Regex> = OnceLock::new();
    let collapsed = re(&SPACES, r"[ \t]{2,}").replace_all(input, " ");
    collapsed.replace('\u{2026}', "...")
}

/// Full cleaning pipeline producing TTS-ready (but not yet abbreviation-expanded)
/// text. See [`crate::normalize`] for abbreviation expansion.
pub fn clean_text(input: &str) -> String {
    let s = decode_entities(input);
    let s = remove_urls(&s);
    let s = remove_emails(&s);
    let s = remove_footnote_markers(&s);
    let s = remove_artifact_lines(&s);
    let s = fix_hyphenation(&s);
    let s = normalize_quotes(&s);
    let s = normalize_punctuation(&s);
    normalize_whitespace(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixes_hyphenated_word_break() {
        assert_eq!(fix_hyphenation("Это краси-\nвый дом"), "Это красивый дом");
    }

    #[test]
    fn removes_urls_and_emails() {
        assert_eq!(remove_urls("see https://example.com/x now").trim(), "see  now".trim());
        assert_eq!(remove_emails("write a@b.com please").trim(), "write  please".trim());
    }

    #[test]
    fn removes_page_numbers_but_keeps_dialogue() {
        let input = "Текст главы.\n42\n— Привет, — сказал он.\n  17  ";
        let out = remove_artifact_lines(input);
        assert!(!out.contains("42"));
        assert!(!out.contains("17"));
        assert!(out.contains("— Привет, — сказал он."));
    }

    #[test]
    fn removes_isbn_and_footnote_markers() {
        assert!(!remove_artifact_lines("ISBN 978-5-00000-000-0").contains("ISBN"));
        assert_eq!(remove_footnote_markers("слово[12] другое"), "слово другое");
    }

    #[test]
    fn normalizes_quotes_keeps_guillemets() {
        assert_eq!(normalize_quotes("“Hi” ‘x’ «нет»"), "\"Hi\" 'x' «нет»");
    }

    #[test]
    fn normalizes_ellipsis_and_spaces() {
        assert_eq!(normalize_punctuation("a…   b"), "a... b");
    }

    #[test]
    fn full_pipeline_preserves_dialogue_and_paragraphs() {
        let input =
            "Глава 1\n\n— Привет, — сказал он.[1]\n\nЭто краси-\nвый день. http://x.io\n\n123\n";
        let out = clean_text(input);
        assert!(out.contains("— Привет, — сказал он."));
        assert!(out.contains("красивый день."));
        assert!(!out.contains("[1]"));
        assert!(!out.contains("http"));
        assert!(!out.contains("\n123"));
        // paragraph boundary between title and dialogue is kept
        assert!(out.contains("Глава 1\n\n— Привет"));
    }

    #[test]
    fn preserves_cyrillic_yo() {
        assert_eq!(clean_text("ёжик съёжился"), "ёжик съёжился");
    }
}
