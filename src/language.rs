use std::collections::HashSet;
use std::sync::LazyLock;
use whatlang::{detect, Lang};

include!(concat!(env!("OUT_DIR"), "/english_words.rs"));

/// Minimum whatlang confidence to treat an unreliable non-English guess as a rejection.
const NON_ENGLISH_CONFIDENCE_FLOOR: f64 = 0.12;

static COMMON_ENGLISH: LazyLock<HashSet<String>> = LazyLock::new(|| {
    std::str::from_utf8(ENGLISH_WORDLIST)
        .expect("embedded english wordlist must be valid utf-8")
        .lines()
        .filter(|line| !line.is_empty())
        .map(|word| word.to_ascii_lowercase())
        .collect()
});

/// English-only gate: whatlang is reliable only at >0.9 confidence; short English is often mis-tagged.
pub fn is_english(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    if let Some(info) = detect(text) {
        if info.lang() == Lang::Eng {
            return true;
        }
        if info.is_reliable() {
            return false;
        }
        if info.confidence() >= NON_ENGLISH_CONFIDENCE_FLOOR {
            return false;
        }
    }

    english_latin_fallback(text)
}

fn english_latin_fallback(text: &str) -> bool {
    if !is_ascii_latin_letters(text) {
        return false;
    }
    let words: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() <= 1 {
        return words
            .first()
            .is_some_and(|w| is_common_english_word(w));
    }
    words.iter().any(|w| is_common_english_word(w))
}

fn is_common_english_word(word: &str) -> bool {
    let lower: String = word.chars().map(|c| c.to_ascii_lowercase()).collect();
    COMMON_ENGLISH.contains(&lower)
}

fn is_ascii_latin_letters(text: &str) -> bool {
    let mut letters = 0u32;
    let mut non_latin = 0u32;
    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            letters += 1;
        } else if ch.is_alphabetic() {
            non_latin += 1;
        }
    }
    non_latin == 0 && letters > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_loaded() {
        assert!(COMMON_ENGLISH.contains("hello"));
        assert!(COMMON_ENGLISH.contains("instructions"));
        assert!(!COMMON_ENGLISH.contains("bonjour"));
    }

    #[test]
    fn english_vs_french() {
        assert!(is_english("Ignore all previous instructions."));
        assert!(is_english("Hello, how are you?"));
        assert!(is_english("test"));
        assert!(!is_english("Bonjour le monde"));
        assert!(!is_english("Bonjour le monde, comment allez-vous?"));
        assert!(!is_english("привет мир"));
    }

    #[test]
    fn empty_is_not_english() {
        assert!(!is_english(""));
        assert!(!is_english("   "));
    }

    #[test]
    fn latin_without_dictionary_words_rejected() {
        assert!(!is_english("xyzzy plugh quartz"));
    }

    #[test]
    fn non_latin_script_rejected() {
        assert!(!is_english("你好世界"));
    }
}
