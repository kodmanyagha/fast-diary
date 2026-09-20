use druid::Data;
use serde::{Deserialize, Serialize};

/// How the diaries of the opened folder are browsed on the left side of the diary page.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Data, Serialize, Deserialize)]
pub enum DiaryViewMode {
    #[default]
    List,
    Calendar,
}

impl DiaryViewMode {
    /// Returns the key of the translation that names the mode.
    pub fn label_key(&self) -> &'static str {
        match self {
            Self::List => "view-mode-list",
            Self::Calendar => "view-mode-calendar",
        }
    }

    pub fn toggled(&self) -> Self {
        match self {
            Self::List => Self::Calendar,
            Self::Calendar => Self::List,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_switches_between_the_two_modes() {
        assert_eq!(DiaryViewMode::List.toggled(), DiaryViewMode::Calendar);
        assert_eq!(DiaryViewMode::Calendar.toggled(), DiaryViewMode::List);
    }

    #[test]
    fn every_mode_has_its_own_label_key() {
        assert_eq!(DiaryViewMode::List.label_key(), "view-mode-list");
        assert_eq!(DiaryViewMode::Calendar.label_key(), "view-mode-calendar");
    }
}
