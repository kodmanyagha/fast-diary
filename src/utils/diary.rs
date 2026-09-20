pub const MAX_DIARY_SUMMARY_LENGTH: usize = 30;

/// Summarizes a diary as its first non-empty line, cut to `MAX_DIARY_SUMMARY_LENGTH` characters.
pub fn summarize(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(MAX_DIARY_SUMMARY_LENGTH).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_has_empty_summary() {
        assert_eq!(summarize(""), "");
        assert_eq!(summarize("  \n\t\n"), "");
    }

    #[test]
    fn summary_is_first_non_empty_line() {
        assert_eq!(summarize("\n\n  Dear diary  \nsecond line"), "Dear diary");
    }

    #[test]
    fn summary_is_cut_by_characters_not_bytes() {
        let summary = summarize(&"ğ".repeat(MAX_DIARY_SUMMARY_LENGTH + 10));

        assert_eq!(summary.chars().count(), MAX_DIARY_SUMMARY_LENGTH);
    }
}
