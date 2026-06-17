//! EPUB extraction: read the zip, follow container.xml → OPF → spine order,
//! strip each XHTML chapter to text.

// Core
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use zip::ZipArchive;
// Services
use super::html::{first_heading, html_to_text};
use super::{local_name_lower, ExtractedBook, Section};

/// Extract an `.epub` file into ordered sections plus title/author metadata.
pub fn extract_epub(bytes: &[u8]) -> Result<ExtractedBook, String> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("invalid epub: {e}"))?;

    let container = read_zip(&mut zip, "META-INF/container.xml")
        .ok_or_else(|| "epub: missing META-INF/container.xml".to_string())?;
    let opf_path = find_rootfile(&container).ok_or_else(|| "epub: no OPF rootfile".to_string())?;

    let opf = read_zip(&mut zip, &opf_path).ok_or_else(|| format!("epub: missing {opf_path}"))?;
    let opf_dir = parent_dir(&opf_path);
    let parsed = parse_opf(&opf);

    let mut sections = Vec::new();
    for idref in &parsed.spine {
        let Some(href) = parsed.manifest.get(idref) else {
            continue;
        };
        let full = join_path(&opf_dir, href);
        if let Some(content) = read_zip(&mut zip, &full) {
            let text = html_to_text(&content);
            if !text.trim().is_empty() {
                sections.push(Section {
                    title: first_heading(&content),
                    text,
                });
            }
        }
    }

    Ok(ExtractedBook {
        title: parsed.title,
        author: parsed.author,
        sections,
    })
}

fn read_zip<R: Read + Seek>(zip: &mut ZipArchive<R>, name: &str) -> Option<String> {
    let mut file = zip.by_name(name).ok()?;
    let mut out = String::new();
    file.read_to_string(&mut out).ok()?;
    Some(out)
}

fn find_rootfile(container_xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(container_xml);
    reader.config_mut().check_end_names = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if local_name_lower(e.name().as_ref()) == "rootfile" {
                    if let Some(path) = attr_value(&e, "full-path") {
                        return Some(path);
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    None
}

#[derive(Default)]
struct Opf {
    title: Option<String>,
    author: Option<String>,
    manifest: HashMap<String, String>,
    spine: Vec<String>,
}

fn parse_opf(opf_xml: &str) -> Opf {
    let mut reader = Reader::from_str(opf_xml);
    reader.config_mut().check_end_names = false;

    let mut opf = Opf::default();
    let mut capturing_title = false;
    let mut capturing_author = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => match local_name_lower(e.name().as_ref()).as_str() {
                "title" => capturing_title = true,
                "creator" => capturing_author = true,
                "item" => register_item(&e, &mut opf),
                "itemref" => register_itemref(&e, &mut opf),
                _ => {}
            },
            Ok(Event::Empty(e)) => match local_name_lower(e.name().as_ref()).as_str() {
                "item" => register_item(&e, &mut opf),
                "itemref" => register_itemref(&e, &mut opf),
                _ => {}
            },
            Ok(Event::Text(e)) => {
                let value = e.unescape().unwrap_or_default();
                if capturing_title && opf.title.is_none() && !value.trim().is_empty() {
                    opf.title = Some(value.trim().to_string());
                } else if capturing_author && opf.author.is_none() && !value.trim().is_empty() {
                    opf.author = Some(value.trim().to_string());
                }
            }
            Ok(Event::End(e)) => match local_name_lower(e.name().as_ref()).as_str() {
                "title" => capturing_title = false,
                "creator" => capturing_author = false,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    opf
}

fn register_item(e: &BytesStart, opf: &mut Opf) {
    if let (Some(id), Some(href)) = (attr_value(e, "id"), attr_value(e, "href")) {
        opf.manifest.insert(id, href);
    }
}

fn register_itemref(e: &BytesStart, opf: &mut Opf) {
    if let Some(idref) = attr_value(e, "idref") {
        opf.spine.push(idref);
    }
}

fn attr_value(e: &BytesStart, key: &str) -> Option<String> {
    for attr in e.attributes().flatten() {
        if local_name_lower(attr.key.as_ref()) == key {
            return attr.unescape_value().ok().map(|v| v.into_owned());
        }
    }
    None
}

fn parent_dir(path: &str) -> String {
    match path.rfind('/') {
        Some(idx) => path[..idx].to_string(),
        None => String::new(),
    }
}

fn join_path(dir: &str, href: &str) -> String {
    let href = href.trim_start_matches("./");
    if dir.is_empty() {
        href.to_string()
    } else {
        format!("{dir}/{href}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn build_epub() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();

            let mut write = |name: &str, body: &str| {
                zip.start_file(name, opts).unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            };

            write("mimetype", "application/epub+zip");
            write(
                "META-INF/container.xml",
                r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#,
            );
            write(
                "OEBPS/content.opf",
                r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Моя книга</dc:title>
    <dc:creator>Анна Смирнова</dc:creator>
  </metadata>
  <manifest>
    <item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="c2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="c1"/>
    <itemref idref="c2"/>
  </spine>
</package>"#,
            );
            write(
                "OEBPS/ch1.xhtml",
                r#"<html><body><h1>Глава первая</h1><p>Привет, мир.</p><p>Второй абзац.</p></body></html>"#,
            );
            write(
                "OEBPS/ch2.xhtml",
                r#"<html><body><h1>Глава вторая</h1><p>Конец.</p></body></html>"#,
            );

            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn extracts_metadata_and_ordered_sections() {
        let epub = build_epub();
        let book = extract_epub(&epub).unwrap();

        assert_eq!(book.title.as_deref(), Some("Моя книга"));
        assert_eq!(book.author.as_deref(), Some("Анна Смирнова"));
        assert_eq!(book.sections.len(), 2);
        assert_eq!(book.sections[0].title.as_deref(), Some("Глава первая"));
        assert_eq!(book.sections[0].text, "Глава первая\n\nПривет, мир.\n\nВторой абзац.");
        assert_eq!(book.sections[1].title.as_deref(), Some("Глава вторая"));
    }

    #[test]
    fn rejects_non_epub_bytes() {
        assert!(extract_epub(b"not a zip").is_err());
    }
}
