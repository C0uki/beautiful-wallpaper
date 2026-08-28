//! Making sense of what the recogniser gives back.
//!
//! Windows' OCR returns a list of lines, not a block of text, and joining them
//! is not the formality it looks like. **A space between two Japanese lines is
//! wrong** — the language does not use them, so the joined text reads as
//! though every line break were a word break, and anything downstream (a
//! translation, a search, the clipboard) inherits the mistake. Which languages
//! that applies to is a fixed, small list, so it lives here under tests rather
//! than being decided at a call site.

/// Whether a language writes without spaces between words.
///
/// Korean is deliberately absent: it is written with spaces, unlike the other
/// three, and joining its lines without one runs the words together.
pub fn is_scriptio_continua(language: &str) -> bool {
    let code = language
        .split(['-', '_'])
        .next()
        .unwrap_or(language)
        .to_lowercase();
    matches!(code.as_str(), "ja" | "zh" | "th" | "lo" | "my" | "km")
}

/// Joins recognised lines into one piece of text.
///
/// Blank lines are dropped rather than kept: the recogniser emits them for
/// gaps in the image, and they are not paragraph breaks in the original.
pub fn join_lines<S: AsRef<str>>(lines: &[S], language: &str) -> String {
    let separator = if is_scriptio_continua(language) {
        ""
    } else {
        " "
    };

    lines
        .iter()
        .map(|line| line.as_ref().trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join(separator)
}

/// Whether there is anything here worth showing.
///
/// An empty result is a real outcome — a selection with no text in it — and
/// has to be told apart from a failure, so the caller can say "nothing to
/// read here" rather than reporting an error that did not happen.
pub fn is_meaningful(text: &str) -> bool {
    text.chars().any(|character| !character.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reason this module exists.
    #[test]
    fn japanese_lines_are_joined_without_spaces() {
        let lines = ["今日は良い", "天気ですね"];
        assert_eq!(join_lines(&lines, "ja"), "今日は良い天気ですね");
        assert_eq!(join_lines(&lines, "ja-JP"), "今日は良い天気ですね");
    }

    #[test]
    fn languages_written_with_spaces_keep_them() {
        let lines = ["the quick", "brown fox"];
        assert_eq!(join_lines(&lines, "en"), "the quick brown fox");
        assert_eq!(join_lines(&lines, "en-GB"), "the quick brown fox");
        assert_eq!(join_lines(&lines, "fr-FR"), "the quick brown fox");
    }

    /// Korean looks like its neighbours and is not written like them.
    #[test]
    fn korean_is_written_with_spaces() {
        assert!(!is_scriptio_continua("ko"));
        assert!(!is_scriptio_continua("ko-KR"));
        assert_eq!(join_lines(&["안녕", "하세요"], "ko"), "안녕 하세요");
    }

    #[test]
    fn the_script_is_read_from_the_language_rather_than_the_region() {
        assert!(is_scriptio_continua("zh-Hans-CN"));
        assert!(is_scriptio_continua("JA"));
        assert!(is_scriptio_continua("ja_JP"));
        assert!(!is_scriptio_continua("de-DE"));
        assert!(!is_scriptio_continua(""));
    }

    #[test]
    fn blank_lines_are_gaps_in_the_image_rather_than_paragraph_breaks() {
        let lines = ["first", "   ", "", "second"];
        assert_eq!(join_lines(&lines, "en"), "first second");
        assert_eq!(join_lines(&lines, "ja"), "firstsecond");
    }

    #[test]
    fn nothing_recognised_gives_nothing_back() {
        let empty: [&str; 0] = [];
        assert_eq!(join_lines(&empty, "en"), "");
        assert_eq!(join_lines(&["", "  "], "en"), "");
    }

    /// A selection with no text in it is an outcome, not a failure.
    #[test]
    fn an_empty_result_is_told_apart_from_a_real_one() {
        assert!(!is_meaningful(""));
        assert!(!is_meaningful("   \n\t "));
        assert!(is_meaningful("a"));
        assert!(is_meaningful(" 今日 "));
    }
}
