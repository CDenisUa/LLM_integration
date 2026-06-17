//! Plain-text extraction: detect encoding, decode, normalize line breaks.

// Core
use chardetng::EncodingDetector;
// Services
use super::{normalize_whitespace, ExtractedBook, Section};

/// Extract a `.txt` file. Encoding is auto-detected (handles UTF-8, Windows-1251,
/// KOI8-R, etc. common for Russian books).
pub fn extract_txt(bytes: &[u8]) -> ExtractedBook {
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);
    let (decoded, _, _) = encoding.decode(bytes);
    let text = normalize_whitespace(&decoded);

    ExtractedBook {
        title: None,
        author: None,
        sections: vec![Section { title: None, text }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_utf8_and_normalizes_line_breaks() {
        let book = extract_txt(b"Hello\r\n\r\n\r\nWorld");
        assert_eq!(book.sections.len(), 1);
        assert_eq!(book.sections[0].text, "Hello\n\nWorld");
    }

    #[test]
    fn decodes_windows_1251_russian() {
        // Build a Windows-1251 byte stream so the test is hermetic.
        let (bytes, _, _) = encoding_rs::WINDOWS_1251.encode("Привет, мир!");
        let book = extract_txt(&bytes);
        assert_eq!(book.sections[0].text, "Привет, мир!");
    }

    #[test]
    fn preserves_cyrillic_yo() {
        let book = extract_txt("ёжик и ёлка".as_bytes());
        assert_eq!(book.sections[0].text, "ёжик и ёлка");
    }
}
