//! Text extraction from book files (txt, epub, fb2).
//!
//! Each extractor returns an [`ExtractedBook`] with optional metadata and the
//! raw `sections` in reading order. Chapter detection/refinement happens later
//! (Phase 5); here we only preserve whatever structure the source provides.

// Services
pub mod epub;
pub mod fb2;
pub mod html;
pub mod txt;

/// One contiguous block of source text (an epub spine item, an fb2 section, or
/// the whole txt file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub title: Option<String>,
    pub text: String,
}

/// Result of extracting a book file, before cleaning/normalization.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedBook {
    pub title: Option<String>,
    pub author: Option<String>,
    pub sections: Vec<Section>,
}

/// Normalize line breaks and whitespace while preserving paragraph boundaries.
///
/// - `\r\n` / `\r` → `\n`
/// - trailing spaces per line are removed
/// - runs of 3+ blank lines collapse to a single blank line (one paragraph gap)
/// - leading/trailing whitespace is trimmed
pub fn normalize_whitespace(input: &str) -> String {
    let unified = input.replace("\r\n", "\n").replace('\r', "\n");
    let joined = unified
        .split('\n')
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = String::with_capacity(joined.len());
    let mut newline_run = 0;
    for ch in joined.chars() {
        if ch == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                out.push('\n');
            }
        } else {
            newline_run = 0;
            out.push(ch);
        }
    }
    out.trim().to_string()
}

/// Lowercased local name (namespace prefix stripped) for an XML/HTML tag or
/// attribute key given as raw bytes.
pub(crate) fn local_name_lower(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    let local = s.rsplit(':').next().unwrap_or(&s);
    local.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_unifies_line_breaks() {
        assert_eq!(normalize_whitespace("a\r\nb\rc"), "a\nb\nc");
    }

    #[test]
    fn normalize_collapses_blank_runs_to_one_gap() {
        assert_eq!(normalize_whitespace("a\n\n\n\nb"), "a\n\nb");
    }

    #[test]
    fn normalize_trims_trailing_spaces_and_edges() {
        assert_eq!(normalize_whitespace("  a   \n  b  \n\n"), "a\n  b");
    }

    #[test]
    fn local_name_strips_namespace() {
        assert_eq!(local_name_lower(b"dc:Title"), "title");
        assert_eq!(local_name_lower(b"P"), "p");
    }
}
