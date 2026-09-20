const ELLIPSIS: &str = ".....";

/// Shortens `text` to at most `max_chars` characters by replacing its middle with five dots, so
/// that the beginning and the end of a long text stay readable. The end keeps one character more
/// than the beginning when the rest cannot be shared equally. A text that is not longer than
/// `max_chars` is returned as it is.
pub fn shorten_in_the_middle(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }

    let kept_chars = max_chars.saturating_sub(ELLIPSIS.len());
    let head_chars = kept_chars / 2;
    let tail_chars = kept_chars - head_chars;

    let head = text.chars().take(head_chars);
    let tail = text.chars().skip(char_count - tail_chars);

    head.chain(ELLIPSIS.chars()).chain(tail).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_text_stays_as_it_is() {
        assert_eq!(
            shorten_in_the_middle("/home/emir/diary", 36),
            "/home/emir/diary"
        );
        assert_eq!(shorten_in_the_middle("12345", 5), "12345");
    }

    #[test]
    fn a_long_text_keeps_its_beginning_and_its_end_around_five_dots() {
        let shortened = shorten_in_the_middle("/home/emir/projects/rust/fast-diary/diaries", 20);

        assert_eq!(shortened, "/home/e...../diaries");
        assert_eq!(shortened.chars().count(), 20);
    }

    #[test]
    fn the_result_is_exactly_as_long_as_allowed() {
        let long_text = "a".repeat(100);

        (6..40).for_each(|max_chars| {
            assert_eq!(
                shorten_in_the_middle(&long_text, max_chars).chars().count(),
                max_chars
            );
        });
    }

    #[test]
    fn the_end_gets_the_odd_character() {
        assert_eq!(shorten_in_the_middle("abcdefghijklmnop", 10), "ab.....nop");
    }

    #[test]
    fn characters_are_counted_and_cut_as_characters_not_bytes() {
        let shortened = shorten_in_the_middle("ğüşiöçĞÜŞİÖÇğüşiöçĞÜŞİÖÇ", 12);

        assert_eq!(shortened, "ğüş.....ŞİÖÇ");
        assert_eq!(shortened.chars().count(), 12);
    }

    #[test]
    fn a_limit_that_only_fits_the_dots_gives_the_dots() {
        assert_eq!(shorten_in_the_middle("a long text", 5), ".....");
        assert_eq!(shorten_in_the_middle("a long text", 0), ".....");
    }
}
