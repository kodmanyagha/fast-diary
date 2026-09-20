use std::{collections::HashMap, sync::LazyLock};

use druid::{widget::LabelText, Env};

use crate::modal::{app_state::AppState, state::language::Language};

const MONTH_KEYS: [&str; 12] = [
    "month-1", "month-2", "month-3", "month-4", "month-5", "month-6", "month-7", "month-8",
    "month-9", "month-10", "month-11", "month-12",
];

const MONTH_SHORT_KEYS: [&str; 12] = [
    "month-short-1",
    "month-short-2",
    "month-short-3",
    "month-short-4",
    "month-short-5",
    "month-short-6",
    "month-short-7",
    "month-short-8",
    "month-short-9",
    "month-short-10",
    "month-short-11",
    "month-short-12",
];

const WEEKDAY_SHORT_KEYS: [&str; 7] = [
    "weekday-short-1",
    "weekday-short-2",
    "weekday-short-3",
    "weekday-short-4",
    "weekday-short-5",
    "weekday-short-6",
    "weekday-short-7",
];

type Texts = HashMap<&'static str, &'static str>;

static ENGLISH_TEXTS: LazyLock<Texts> =
    LazyLock::new(|| parse_texts(include_str!("../../resources/i18n/en-US/builtin.ftl")));
static TURKISH_TEXTS: LazyLock<Texts> =
    LazyLock::new(|| parse_texts(include_str!("../../resources/i18n/tr-TR/builtin.ftl")));

/// Reads the `key = text` lines of a translation file.
fn parse_texts(source: &'static str) -> Texts {
    source
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with(char::is_whitespace))
        .filter_map(|line| line.split_once(" = "))
        .map(|(key, text)| (key.trim(), text.trim()))
        .collect()
}

/// Looks `key` up in the translations of `language`. A text that is missing in the language is
/// taken from English, and a text that no language has is shown as its key.
pub fn translate(language: Language, key: &str) -> String {
    let texts_of_language = match language.resolved() {
        Language::Turkish => &TURKISH_TEXTS,
        Language::English | Language::System => &ENGLISH_TEXTS,
    };

    texts_of_language
        .get(key)
        .or_else(|| ENGLISH_TEXTS.get(key))
        .map_or_else(|| key.to_string(), |text| text.to_string())
}

/// Builds a text that is translated into the language of the app state whenever it is drawn, so
/// that it changes as soon as another language is chosen.
pub fn text(key: &'static str) -> LabelText<AppState> {
    LabelText::from(move |app_state: &AppState, _env: &Env| translate(app_state.language, key))
}

fn translate_indexed(language: Language, keys: &[&'static str], index: usize) -> String {
    keys.get(index)
        .map(|key| translate(language, key))
        .unwrap_or_default()
}

/// Returns the name of the month `month`, counted from 1 for January.
pub fn month_name(language: Language, month: u32) -> String {
    translate_indexed(language, &MONTH_KEYS, (month as usize).wrapping_sub(1))
}

/// Returns the short name of the month `month`, counted from 1 for January.
pub fn month_short_name(language: Language, month: u32) -> String {
    translate_indexed(
        language,
        &MONTH_SHORT_KEYS,
        (month as usize).wrapping_sub(1),
    )
}

/// Returns the short name of the weekday at `index`, counted from 0 for Monday.
pub fn weekday_short_name(language: Language, index: usize) -> String {
    translate_indexed(language, &WEEKDAY_SHORT_KEYS, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_are_taken_from_the_chosen_language() {
        assert_eq!(translate(Language::English, "menu-file"), "File");
        assert_eq!(translate(Language::Turkish, "menu-file"), "Dosya");
    }

    #[test]
    fn a_text_that_a_language_lacks_is_taken_from_english() {
        assert_eq!(
            translate(Language::Turkish, "hello-counter"),
            translate(Language::English, "hello-counter")
        );
    }

    #[test]
    fn a_text_that_no_language_has_is_shown_as_its_key() {
        assert_eq!(translate(Language::Turkish, "no-such-text"), "no-such-text");
    }

    #[test]
    fn month_names_follow_the_month_number_and_the_language() {
        assert_eq!(month_name(Language::English, 10), "October");
        assert_eq!(month_name(Language::Turkish, 10), "Ekim");
        assert_eq!(month_short_name(Language::English, 10), "Oct");
        assert_eq!(month_short_name(Language::Turkish, 10), "Eki");
        assert_eq!(month_name(Language::Turkish, 0), "");
        assert_eq!(month_name(Language::Turkish, 13), "");
    }

    #[test]
    fn weekday_names_follow_the_index_and_the_language() {
        assert_eq!(weekday_short_name(Language::English, 0), "Mo");
        assert_eq!(weekday_short_name(Language::Turkish, 0), "Pt");
        assert_eq!(weekday_short_name(Language::Turkish, 6), "Pz");
        assert_eq!(weekday_short_name(Language::Turkish, 7), "");
    }

    #[test]
    fn comments_and_continuation_lines_are_not_texts() {
        let texts = parse_texts("# note\nkey = value\n  more = ignored\nempty line\n");

        assert_eq!(texts.get("key"), Some(&"value"));
        assert_eq!(texts.len(), 1);
    }
}
