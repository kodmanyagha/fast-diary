use serde::{Deserialize, Serialize};

const DEFAULT_LIST_SPLIT_PERCENT: f64 = 30.0;
const DEFAULT_EDITOR_SPLIT_PERCENT: f64 = 50.0;
const MIN_SPLIT_PERCENT: f64 = 5.0;
const MAX_SPLIT_PERCENT: f64 = 95.0;
const PERCENT_DECIMALS_FACTOR: f64 = 100.0;

/// How the diary page is divided, stored as the percentage of the width that the first part
/// takes: the diary list next to the editor, and the editor next to the preview.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayoutSettings {
    #[serde(default = "default_list_split_percent")]
    pub list_split_percent: f64,
    #[serde(default = "default_editor_split_percent")]
    pub editor_split_percent: f64,
}

fn default_list_split_percent() -> f64 {
    DEFAULT_LIST_SPLIT_PERCENT
}

fn default_editor_split_percent() -> f64 {
    DEFAULT_EDITOR_SPLIT_PERCENT
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            list_split_percent: DEFAULT_LIST_SPLIT_PERCENT,
            editor_split_percent: DEFAULT_EDITOR_SPLIT_PERCENT,
        }
    }
}

impl LayoutSettings {
    /// Builds the settings from split positions given as fractions of the width (0.3 is 30%).
    pub fn from_ratios(list_split_ratio: f64, editor_split_ratio: f64) -> Self {
        Self {
            list_split_percent: Self::ratio_to_percent(list_split_ratio),
            editor_split_percent: Self::ratio_to_percent(editor_split_ratio),
        }
    }

    pub fn list_split_ratio(&self) -> f64 {
        Self::percent_to_ratio(self.list_split_percent, DEFAULT_LIST_SPLIT_PERCENT)
    }

    pub fn editor_split_ratio(&self) -> f64 {
        Self::percent_to_ratio(self.editor_split_percent, DEFAULT_EDITOR_SPLIT_PERCENT)
    }

    fn ratio_to_percent(ratio: f64) -> f64 {
        (ratio * 100.0 * PERCENT_DECIMALS_FACTOR).round() / PERCENT_DECIMALS_FACTOR
    }

    fn percent_to_ratio(percent: f64, default_percent: f64) -> f64 {
        let usable_percent = if percent.is_finite() {
            percent.clamp(MIN_SPLIT_PERCENT, MAX_SPLIT_PERCENT)
        } else {
            default_percent
        };

        usable_percent / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_split_the_page_thirty_and_fifty_percent() {
        let layout = LayoutSettings::default();

        assert_eq!(layout.list_split_ratio(), 0.3);
        assert_eq!(layout.editor_split_ratio(), 0.5);
    }

    #[test]
    fn ratios_are_stored_as_rounded_percentages() {
        let layout = LayoutSettings::from_ratios(0.30000000000000004, 0.123456);

        assert_eq!(layout.list_split_percent, 30.0);
        assert_eq!(layout.editor_split_percent, 12.35);
    }

    #[test]
    fn unusable_percentages_fall_back_or_are_clamped() {
        let layout = LayoutSettings {
            list_split_percent: f64::NAN,
            editor_split_percent: 250.0,
        };

        assert_eq!(layout.list_split_ratio(), 0.3);
        assert_eq!(layout.editor_split_ratio(), 0.95);
        assert_eq!(
            LayoutSettings {
                list_split_percent: -3.0,
                editor_split_percent: 0.0
            }
            .list_split_ratio(),
            0.05
        );
    }

    #[test]
    fn missing_fields_load_as_defaults() -> serde_json::Result<()> {
        let layout: LayoutSettings = serde_json::from_str(r#"{"list_split_percent": 42.5}"#)?;

        assert_eq!(layout.list_split_percent, 42.5);
        assert_eq!(layout.editor_split_percent, 50.0);
        Ok(())
    }
}
