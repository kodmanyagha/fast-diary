use std::ops::Range;

pub const MAX_DIARY_SUMMARY_LENGTH: usize = 30;

/// Summarizes a diary as its first non-empty line, cut to `MAX_DIARY_SUMMARY_LENGTH` characters.
pub fn summarize(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(MAX_DIARY_SUMMARY_LENGTH).collect())
        .unwrap_or_default()
}

/// Returns `text` with `replaced` swapped for `block`, which is put on a line of its own, and
/// the offset right after it. A range that does not fall on character boundaries of `text` is
/// treated as the end of the text.
pub fn with_block_inserted(text: &str, replaced: Range<usize>, block: &str) -> (String, usize) {
    let (before, after) = text
        .get(..replaced.start)
        .zip(text.get(replaced.end..))
        .filter(|_| replaced.start <= replaced.end)
        .unwrap_or((text, ""));
    let leading_break = if before.is_empty() || before.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    let trailing_break = if after.starts_with('\n') { "" } else { "\n" };
    let inserted = [leading_break, block, trailing_break].concat();

    (
        [before, &inserted, after].concat(),
        before.len() + inserted.len(),
    )
}

/// Builds the markdown that shows the image at `reference`, with `description` as its
/// alternative text.
pub fn image_markdown(description: &str, reference: &str) -> String {
    let alternative_text = description
        .chars()
        .filter(|character| !matches!(character, '[' | ']' | '\n' | '\r'))
        .collect::<String>();

    format!("![{}]({reference})", alternative_text.trim())
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

    #[test]
    fn a_block_typed_inside_a_line_gets_a_line_of_its_own() {
        assert_eq!(
            with_block_inserted("before after", 7..7, "B"),
            ("before \nB\nafter".to_string(), 10)
        );
    }

    #[test]
    fn a_block_at_the_start_of_a_line_adds_no_empty_line() {
        assert_eq!(
            with_block_inserted("one\ntwo", 4..4, "B"),
            ("one\nB\ntwo".to_string(), 6)
        );
        assert_eq!(with_block_inserted("", 0..0, "B"), ("B\n".to_string(), 2));
    }

    #[test]
    fn a_block_before_a_line_break_reuses_it() {
        assert_eq!(
            with_block_inserted("one\n", 3..3, "B"),
            ("one\nB\n".to_string(), 5)
        );
    }

    #[test]
    fn a_block_replaces_the_selected_text() {
        assert_eq!(
            with_block_inserted("keep DROP keep", 5..9, "B"),
            ("keep \nB\n keep".to_string(), 8)
        );
    }

    #[test]
    fn a_range_inside_a_character_puts_the_block_at_the_end() {
        assert_eq!(
            with_block_inserted("ğ", 1..1, "B"),
            ("ğ\nB\n".to_string(), 5)
        );
        assert_eq!(
            with_block_inserted("ab", 5..9, "B"),
            ("ab\nB\n".to_string(), 5)
        );
    }

    #[test]
    fn image_markdown_keeps_its_brackets_balanced() {
        assert_eq!(
            image_markdown(" Tatil [1] ", "assets/2511/tatil_1.png"),
            "![Tatil 1](assets/2511/tatil_1.png)"
        );
    }
}
