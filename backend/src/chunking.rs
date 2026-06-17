//! Split chapter text into TTS-sized chunks without cutting mid-sentence.
//!
//! Strategy: pack whole paragraphs up to `max_chars`; if a single paragraph is
//! too large, fall back to packing whole sentences. A single sentence longer
//! than `max_chars` is emitted on its own (never split mid-sentence).

/// Default maximum characters per chunk (spec range 1500–3000).
pub const DEFAULT_MAX_CHUNK_CHARS: usize = 2500;

fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Split `text` into ordered chunks no larger than `max_chars` (best effort).
pub fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut chunks: Vec<String> = Vec::new();
    let mut cur = String::new();

    let mut flush = |cur: &mut String, chunks: &mut Vec<String>| {
        let trimmed = cur.trim();
        if !trimmed.is_empty() {
            chunks.push(trimmed.to_string());
        }
        cur.clear();
    };

    for para in text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        let fits = char_len(&cur) + char_len(para) + 2 <= max_chars;
        if fits {
            if !cur.is_empty() {
                cur.push_str("\n\n");
            }
            cur.push_str(para);
            continue;
        }

        flush(&mut cur, &mut chunks);

        if char_len(para) <= max_chars {
            cur.push_str(para);
            continue;
        }

        // Paragraph too big: pack whole sentences.
        for sentence in split_sentences(para) {
            if !cur.is_empty() && char_len(&cur) + char_len(&sentence) + 1 > max_chars {
                flush(&mut cur, &mut chunks);
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(&sentence);
        }
    }

    flush(&mut cur, &mut chunks);
    chunks
}

/// Split text into sentences. A sentence ends at `. ! ? …` (and any run of those)
/// plus optional closing quotes/brackets, followed by whitespace or end-of-text.
pub fn split_sentences(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut sentences = Vec::new();
    let mut start = 0;
    let mut i = 0;

    while i < chars.len() {
        if matches!(chars[i], '.' | '!' | '?' | '…') {
            let mut j = i + 1;
            while j < chars.len() && matches!(chars[j], '.' | '!' | '?' | '…') {
                j += 1;
            }
            while j < chars.len() && matches!(chars[j], '"' | '»' | '\'' | '\u{201D}' | ')' | ']')
            {
                j += 1;
            }
            if j >= chars.len() || chars[j].is_whitespace() {
                let sentence: String = chars[start..j].iter().collect();
                let trimmed = sentence.trim();
                if !trimmed.is_empty() {
                    sentences.push(trimmed.to_string());
                }
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                start = j;
                i = j;
                continue;
            }
            i = j;
        } else {
            i += 1;
        }
    }

    if start < chars.len() {
        let tail: String = chars[start..].iter().collect();
        let trimmed = tail.trim();
        if !trimmed.is_empty() {
            sentences.push(trimmed.to_string());
        }
    }
    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_single_chunk() {
        assert_eq!(chunk_text("Короткий текст.", 100), vec!["Короткий текст."]);
    }

    #[test]
    fn combines_paragraphs_under_limit() {
        let text = "Абзац один.\n\nАбзац два.";
        assert_eq!(chunk_text(text, 100), vec!["Абзац один.\n\nАбзац два."]);
    }

    #[test]
    fn splits_long_paragraph_by_sentences_without_cutting() {
        let text = "Первое предложение. Второе предложение. Третье предложение.";
        let chunks = chunk_text(text, 25);
        assert_eq!(chunks.len(), 3);
        // every chunk ends at a sentence boundary
        assert!(chunks.iter().all(|c| c.ends_with('.')));
        // order and content preserved
        assert_eq!(chunks.join(" "), text);
    }

    #[test]
    fn respects_max_chars_when_possible() {
        let text = "Раз два три. Четыре пять шесть. Семь восемь девять.";
        let chunks = chunk_text(text, 20);
        assert!(chunks.iter().all(|c| c.chars().count() <= 20));
    }

    #[test]
    fn split_sentences_handles_punctuation_variants() {
        let s = split_sentences("Привет! Как дела? Хорошо… Конец.");
        assert_eq!(s, vec!["Привет!", "Как дела?", "Хорошо…", "Конец."]);
    }

    #[test]
    fn oversized_single_sentence_is_emitted_alone() {
        let text = "ОченьДлинноеСловоБезПробеловКотороеНельзяРазбить.";
        let chunks = chunk_text(text, 10);
        assert_eq!(chunks, vec![text]);
    }
}
