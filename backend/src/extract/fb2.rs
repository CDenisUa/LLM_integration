//! FB2 (FictionBook 2) extraction: XML parse → title/author + body sections.

// Core
use chardetng::EncodingDetector;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
// Services
use super::{local_name_lower, normalize_whitespace, ExtractedBook, Section};

#[derive(Default)]
struct SectionBuilder {
    title: String,
    text: String,
}

#[derive(PartialEq)]
enum Capture {
    None,
    BookTitle,
    FirstName,
    LastName,
}

/// Extract a `.fb2` file. Each `<section>` becomes a [`Section`]; nested
/// sections are flattened in document order.
pub fn extract_fb2(bytes: &[u8]) -> Result<ExtractedBook, String> {
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let (xml, _, _) = detector.guess(None, true).decode(bytes);

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().check_end_names = false;

    let mut title: Option<String> = None;
    let mut first_name = String::new();
    let mut last_name = String::new();
    let mut author_done = false;

    let mut in_title_info = false;
    let mut in_author = false;
    let mut in_body = false;
    let mut in_section_title = false;
    let mut in_paragraph = false;
    let mut capture = Capture::None;

    let mut stack: Vec<SectionBuilder> = Vec::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut book_title_buf = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = local_name_lower(e.name().as_ref());
                match name.as_str() {
                    "title-info" => in_title_info = true,
                    "author" if in_title_info => in_author = true,
                    "book-title" if in_title_info => {
                        capture = Capture::BookTitle;
                        book_title_buf.clear();
                    }
                    "first-name" if in_author && !author_done => capture = Capture::FirstName,
                    "last-name" if in_author && !author_done => capture = Capture::LastName,
                    "body" => in_body = true,
                    "section" if in_body => stack.push(SectionBuilder::default()),
                    "title" if in_body && !stack.is_empty() => in_section_title = true,
                    "p" if in_body && !stack.is_empty() => in_paragraph = true,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let value = e.unescape().unwrap_or_default();
                match capture {
                    Capture::BookTitle => book_title_buf.push_str(&value),
                    Capture::FirstName => first_name.push_str(value.trim()),
                    Capture::LastName => last_name.push_str(value.trim()),
                    Capture::None => {
                        if in_paragraph {
                            if let Some(cur) = stack.last_mut() {
                                if in_section_title {
                                    cur.title.push_str(&value);
                                } else {
                                    cur.text.push_str(&value);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = local_name_lower(e.name().as_ref());
                match name.as_str() {
                    "title-info" => in_title_info = false,
                    "author" if in_title_info => {
                        in_author = false;
                        if !first_name.is_empty() || !last_name.is_empty() {
                            author_done = true;
                        }
                    }
                    "book-title" => {
                        if !book_title_buf.trim().is_empty() {
                            title = Some(book_title_buf.trim().to_string());
                        }
                        capture = Capture::None;
                    }
                    "first-name" | "last-name" => capture = Capture::None,
                    "p" if in_paragraph => {
                        if let Some(cur) = stack.last_mut() {
                            if in_section_title {
                                cur.title.push(' ');
                            } else {
                                cur.text.push_str("\n\n");
                            }
                        }
                        in_paragraph = false;
                    }
                    "title" if in_section_title => in_section_title = false,
                    "section" if in_body => {
                        if let Some(builder) = stack.pop() {
                            let sec_title = builder.title.split_whitespace().collect::<Vec<_>>().join(" ");
                            let sec_text = normalize_whitespace(&builder.text);
                            if !sec_title.is_empty() || !sec_text.is_empty() {
                                sections.push(Section {
                                    title: (!sec_title.is_empty()).then_some(sec_title),
                                    text: sec_text,
                                });
                            }
                        }
                    }
                    "body" => in_body = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(format!("FB2 parse error: {err}")),
            _ => {}
        }
    }

    let author = match (first_name.trim(), last_name.trim()) {
        ("", "") => None,
        (f, "") => Some(f.to_string()),
        ("", l) => Some(l.to_string()),
        (f, l) => Some(format!("{f} {l}")),
    };

    Ok(ExtractedBook {
        title,
        author,
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info>
      <book-title>Тестовая книга</book-title>
      <author><first-name>Иван</first-name><last-name>Петров</last-name></author>
    </title-info>
  </description>
  <body>
    <section>
      <title><p>Глава 1</p></title>
      <p>Первый абзац.</p>
      <p>Второй абзац.</p>
    </section>
    <section>
      <title><p>Глава 2</p></title>
      <p>Текст главы два.</p>
    </section>
  </body>
</FictionBook>"#;

    #[test]
    fn extracts_metadata() {
        let book = extract_fb2(SAMPLE.as_bytes()).unwrap();
        assert_eq!(book.title.as_deref(), Some("Тестовая книга"));
        assert_eq!(book.author.as_deref(), Some("Иван Петров"));
    }

    #[test]
    fn extracts_sections_with_titles_and_paragraphs() {
        let book = extract_fb2(SAMPLE.as_bytes()).unwrap();
        assert_eq!(book.sections.len(), 2);
        assert_eq!(book.sections[0].title.as_deref(), Some("Глава 1"));
        assert_eq!(book.sections[0].text, "Первый абзац.\n\nВторой абзац.");
        assert_eq!(book.sections[1].title.as_deref(), Some("Глава 2"));
        assert_eq!(book.sections[1].text, "Текст главы два.");
    }
}
