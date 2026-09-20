use std::sync::OnceLock;

use druid::{Application, Data};
use serde::{Deserialize, Serialize};

/// The language of the texts of the app. `System` follows the language of the operating system.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Data, Serialize, Deserialize)]
pub enum Language {
    #[default]
    System,
    English,
    Turkish,
}

impl Language {
    /// Returns the language that is really used: for `System` it is the language of the operating
    /// system, or English when the app has no translations for it.
    pub fn resolved(&self) -> Language {
        match self {
            Language::System => Self::system_language(),
            chosen => *chosen,
        }
    }

    /// Picks the language that belongs to a locale name such as `tr-TR` or `tr_TR.UTF-8`.
    pub fn from_locale_name(locale_name: &str) -> Language {
        if locale_name.to_ascii_lowercase().starts_with("tr") {
            Language::Turkish
        } else {
            Language::English
        }
    }

    fn system_language() -> Language {
        static SYSTEM_LANGUAGE: OnceLock<Language> = OnceLock::new();

        *SYSTEM_LANGUAGE.get_or_init(|| Self::from_locale_name(&Application::get_locale()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_names_are_mapped_to_the_languages_of_the_app() {
        assert_eq!(Language::from_locale_name("tr"), Language::Turkish);
        assert_eq!(Language::from_locale_name("tr-TR"), Language::Turkish);
        assert_eq!(Language::from_locale_name("tr_TR.UTF-8"), Language::Turkish);
        assert_eq!(Language::from_locale_name("en-US"), Language::English);
        assert_eq!(Language::from_locale_name("de-DE"), Language::English);
        assert_eq!(Language::from_locale_name(""), Language::English);
    }

    #[test]
    fn a_chosen_language_is_used_as_it_is() {
        assert_eq!(Language::Turkish.resolved(), Language::Turkish);
        assert_eq!(Language::English.resolved(), Language::English);
    }
}
