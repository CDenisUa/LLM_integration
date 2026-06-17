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
}
