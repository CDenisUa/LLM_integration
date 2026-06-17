//! TTS normalization: expand common Russian/English abbreviations so the engine
//! pronounces them naturally. Produces the "normalized" text variant; the
//! cleaned text is kept separately (see [`crate::clean`]).
//!
//! Numbers are intentionally left as digits for now (number-to-words is a future
//! improvement). `г.` is ambiguous (год/город) and left unchanged by default.

// Core
use std::sync::OnceLock;
use regex::Regex;

type Rule = (Regex, &'static str);

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        let raw: &[(&str, &str)] = &[
            // Russian — multiword first so they win over their sub-parts.
            (r"(?i)\bи\s+т\.\s*д\.", "и так далее"),
            (r"(?i)\bи\s+т\.\s*п\.", "и тому подобное"),
            (r"(?i)\bт\.\s*е\.", "то есть"),
            (r"(?i)\bт\.\s*к\.", "так как"),
            (r"(?i)\bул\.", "улица"),
            (r"(?i)\bстр\.", "страница"),
            // English.
            (r"(?i)\be\.\s*g\.", "for example"),
            (r"(?i)\bi\.\s*e\.", "that is"),
            (r"\bMrs\.", "Misses"),
            (r"\bMr\.", "Mister"),
            (r"\bDr\.", "Doctor"),
        ];
        raw.iter()
            .map(|(p, r)| (Regex::new(p).expect("valid regex"), *r))
            .collect()
    })
}

/// Expand abbreviations in `input` for TTS. `№` becomes "номер".
pub fn normalize_for_tts(input: &str) -> String {
    let mut text = input.to_string();
    for (re, replacement) in rules() {
        text = re.replace_all(&text, *replacement).into_owned();
    }
    text.replace('№', "номер ")
        .replace("  ", " ") // tidy any double spaces introduced above
}

fn cached(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("valid regex"))
}

/// Punctuation that is safe to read aloud. Everything else (except letters,
/// digits and whitespace) is dropped by [`sanitize_for_speech`].
fn is_speech_punctuation(c: char) -> bool {
    matches!(
        c,
        ',' | '!' | '?' | ';' | ':' | '(' | ')' | '«' | '»' | '"' | '\'' | '—' | '–' | '-'
    )
}

/// Sanitize realtime-reader text before sending it to the TTS engine.
///
/// Abbreviations are expanded first (they rely on their dotted form), then we
/// drop code/table tokens and other special characters, and finally remove
/// every dot/ellipsis — the engine would otherwise vocalize sentence periods
/// ("точка"). Dots become commas so natural pauses are preserved without the
/// spoken artifact. Reduces the character count handed to the model.
pub fn sanitize_for_speech(input: &str) -> String {
    // 1. Expand abbreviations (т.е., и т.д., Mr. …) before any dots are touched.
    let text = normalize_for_tts(input);

    // 2. Drop whole tokens carrying programming/table-drawing symbols — code,
    //    formulas and table cells are noise when read aloud.
    static CODEY: OnceLock<Regex> = OnceLock::new();
    let text = cached(&CODEY, r"\S*[{}\[\]<>|/\\=`~^*_\x{2502}\x{2500}\x{251C}\x{2524}\x{252C}\x{2534}\x{253C}]\S*")
        .replace_all(&text, " ")
        .into_owned();

    // 3. Ellipses and dot runs (incl. dotted leaders in tables of contents) and
    //    single sentence dots → a comma, i.e. a pause the engine never speaks.
    static DOTS: OnceLock<Regex> = OnceLock::new();
    let text = cached(&DOTS, r"[.\x{2026}]+").replace_all(&text, ",").into_owned();

    // 4. Whitelist: keep letters, digits, whitespace and speech-safe punctuation;
    //    every other special character becomes a space.
    let text: String = text
        .chars()
        .map(|c| {
            if c.is_alphabetic() || c.is_numeric() || c.is_whitespace() || is_speech_punctuation(c) {
                c
            } else {
                ' '
            }
        })
        .collect();

    // 5. Tidy: collapse whitespace, glue punctuation to its word, dedupe commas,
    //    and trim dangling separators at the edges.
    static WS: OnceLock<Regex> = OnceLock::new();
    let text = cached(&WS, r"\s+").replace_all(&text, " ").into_owned();
    static PUNCT_SPACE: OnceLock<Regex> = OnceLock::new();
    let text = cached(&PUNCT_SPACE, r"\s+([,!?;:])")
        .replace_all(&text, "$1")
        .into_owned();
    static MULTI_COMMA: OnceLock<Regex> = OnceLock::new();
    let text = cached(&MULTI_COMMA, r"(?:,\s*){2,}")
        .replace_all(&text, ", ")
        .into_owned();

    text.trim()
        .trim_matches(|c: char| matches!(c, ',' | ' ' | ';' | ':' | '-' | '—' | '–'))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_russian_abbreviations() {
        assert_eq!(normalize_for_tts("т.е. это так"), "то есть это так");
        assert_eq!(normalize_for_tts("это так, т.к. надо"), "это так, так как надо");
        assert_eq!(
            normalize_for_tts("яблоки, груши и т.д."),
            "яблоки, груши и так далее"
        );
        assert_eq!(
            normalize_for_tts("книги и т.п. вещи"),
            "книги и тому подобное вещи"
        );
        assert_eq!(normalize_for_tts("ул. Ленина"), "улица Ленина");
        assert_eq!(normalize_for_tts("стр. 5"), "страница 5");
    }

    #[test]
    fn expands_english_abbreviations() {
        assert_eq!(normalize_for_tts("e.g. apples"), "for example apples");
        assert_eq!(normalize_for_tts("i.e. that"), "that is that");
        assert_eq!(normalize_for_tts("Mr. Smith"), "Mister Smith");
        assert_eq!(normalize_for_tts("Mrs. Smith"), "Misses Smith");
        assert_eq!(normalize_for_tts("Dr. House"), "Doctor House");
    }

    #[test]
    fn expands_number_sign() {
        assert_eq!(normalize_for_tts("№5").trim(), "номер 5");
    }

    #[test]
    fn leaves_ambiguous_and_plain_text_untouched() {
        // `г.` stays as-is (год vs город is context-dependent).
        assert_eq!(normalize_for_tts("в 1990 г. было"), "в 1990 г. было");
        assert_eq!(normalize_for_tts("обычный текст"), "обычный текст");
    }

    #[test]
    fn sanitize_replaces_dots_and_ellipsis_with_pause() {
        assert_eq!(
            sanitize_for_speech("Привет. Как дела... нормально"),
            "Привет, Как дела, нормально"
        );
        // No trailing dot artifact at the end.
        assert_eq!(sanitize_for_speech("Конец."), "Конец");
    }

    #[test]
    fn sanitize_drops_code_and_table_tokens() {
        assert_eq!(sanitize_for_speech("вот код foo()=bar; и текст"), "вот код и текст");
        assert_eq!(sanitize_for_speech("Имя | Возраст | Город"), "Имя Возраст Город");
    }

    #[test]
    fn sanitize_strips_special_characters_but_keeps_numbers() {
        assert_eq!(sanitize_for_speech("цена 50$ и #1"), "цена 50 и 1");
    }

    #[test]
    fn sanitize_still_expands_abbreviations() {
        assert_eq!(sanitize_for_speech("т.е. так"), "то есть так");
    }

    #[test]
    fn sanitize_empty_when_only_noise() {
        assert_eq!(sanitize_for_speech("......"), "");
        assert_eq!(sanitize_for_speech("{ }"), "");
    }
}
