//! Convert (X)HTML chapter markup to plain text, preserving paragraph breaks.

// Core
use quick_xml::events::Event;
use quick_xml::reader::Reader;
// Services
use super::{local_name_lower, normalize_whitespace};

const BLOCK_TAGS: [&str; 12] = [
    "p", "div", "li", "h1", "h2", "h3", "h4", "h5", "h6", "tr", "blockquote", "section",
];
const SKIP_TAGS: [&str; 4] = ["script", "style", "head", "title"];

/// Strip HTML tags and return readable text. Block-level elements become
/// paragraph breaks; `<br>` becomes a single line break.
pub fn html_to_text(html: &str) -> String {
    let mut reader = Reader::from_str(html);
    reader.config_mut().check_end_names = false;

    let mut text = String::new();
    let mut skip_depth: usize = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = local_name_lower(e.name().as_ref());
                if SKIP_TAGS.contains(&name.as_str()) {
                    skip_depth += 1;
                } else if name == "br" {
                    text.push('\n');
                }
            }
            Ok(Event::Empty(e)) => {
                let name = local_name_lower(e.name().as_ref());
                if name == "br" {
                    text.push('\n');
                }
            }
            Ok(Event::End(e)) => {
                let name = local_name_lower(e.name().as_ref());
                if SKIP_TAGS.contains(&name.as_str()) {
                    skip_depth = skip_depth.saturating_sub(1);
                } else if BLOCK_TAGS.contains(&name.as_str()) {
                    text.push_str("\n\n");
                }
            }
            Ok(Event::Text(e)) => {
                if skip_depth == 0 {
                    text.push_str(&e.unescape().unwrap_or_default());
                }
            }
            Ok(Event::CData(e)) => {
                if skip_depth == 0 {
                    text.push_str(&String::from_utf8_lossy(&e));
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    normalize_whitespace(&text)
}

/// First heading (`<h1>`..`<h3>`) text, used as a section/chapter title hint.
pub fn first_heading(html: &str) -> Option<String> {
    let mut reader = Reader::from_str(html);
    reader.config_mut().check_end_names = false;

    let mut capturing = false;
    let mut buf = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = local_name_lower(e.name().as_ref());
                if matches!(name.as_str(), "h1" | "h2" | "h3") {
                    capturing = true;
                }
            }
            Ok(Event::Text(e)) if capturing => {
                buf.push_str(&e.unescape().unwrap_or_default());
            }
            Ok(Event::End(e)) => {
                let name = local_name_lower(e.name().as_ref());
                if matches!(name.as_str(), "h1" | "h2" | "h3") && capturing {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    let title = buf.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_become_blank_line_separated() {
        assert_eq!(
            html_to_text("<p>Hello</p><p>World</p>"),
            "Hello\n\nWorld"
        );
    }

    #[test]
    fn decodes_entities_and_headings() {
        assert_eq!(
            html_to_text("<h1>Title</h1><p>Body &amp; more</p>"),
            "Title\n\nBody & more"
        );
    }

    #[test]
    fn br_is_single_line_break() {
        assert_eq!(html_to_text("<div>a<br/>b</div>"), "a\nb");
    }

    #[test]
    fn script_and_style_are_dropped() {
        assert_eq!(
            html_to_text("<style>p{color:red}</style><p>Visible</p><script>x=1</script>"),
            "Visible"
        );
    }

    #[test]
    fn first_heading_returns_title() {
        assert_eq!(
            first_heading("<html><body><h2>Глава 1</h2><p>text</p></body></html>"),
            Some("Глава 1".to_string())
        );
        assert_eq!(first_heading("<p>no heading</p>"), None);
    }
}
