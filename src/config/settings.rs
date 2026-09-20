use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

use super::{layout_settings::LayoutSettings, window_settings::WindowSettings};
use crate::{
    modal::{
        app_state::AppState,
        state::{diary_view_mode::DiaryViewMode, editor_mode::EditorMode, language::Language},
    },
    storage::atomic_write::write_atomically,
};

const SETTINGS_DIR_NAME: &str = "fast-diary";
const SETTINGS_FILE_NAME: &str = "settings.json";
const MAX_RECENT_FOLDERS: usize = 10;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub recent_folders: Vec<String>,
    #[serde(default)]
    pub diary_view_mode: DiaryViewMode,
    #[serde(default)]
    pub editor_mode: EditorMode,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub window: WindowSettings,
    #[serde(default)]
    pub layout: LayoutSettings,
}

impl From<&AppState> for Settings {
    fn from(app_state: &AppState) -> Self {
        Self {
            recent_folders: app_state.recent_folders.iter().cloned().collect(),
            diary_view_mode: app_state.diary_view_mode,
            editor_mode: app_state.editor_mode,
            language: app_state.language,
            window: app_state.window,
            layout: LayoutSettings::from_ratios(
                app_state.list_split_ratio,
                app_state.editor_split_ratio,
            ),
        }
    }
}

impl Settings {
    fn settings_file_path() -> anyhow::Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or(anyhow!("Could not resolve config directory."))?;

        Ok(config_dir.join(SETTINGS_DIR_NAME).join(SETTINGS_FILE_NAME))
    }

    pub fn load() -> Self {
        Self::try_load().unwrap_or_else(|err| {
            tracing::warn!("Could not load settings, using defaults: {err}");
            Settings::default()
        })
    }

    fn try_load() -> anyhow::Result<Self> {
        let path = Self::settings_file_path()?;
        let content = fs::read_to_string(path)?;
        let settings = serde_json::from_str(&content)?;

        Ok(settings)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::settings_file_path()?;
        let parent = path
            .parent()
            .ok_or(anyhow!("Settings file path has no parent directory."))?;

        fs::create_dir_all(parent).context("Could not create settings directory.")?;

        let content = serde_json::to_string_pretty(self)?;
        write_atomically(&path, content.as_bytes())?;

        Ok(())
    }

    pub fn push_recent_folder(&mut self, folder_path: String) {
        self.recent_folders.retain(|item| item != &folder_path);
        self.recent_folders.insert(0, folder_path);
        self.recent_folders.truncate(MAX_RECENT_FOLDERS);
    }
}

#[cfg(test)]
mod tests {
    use druid::{Point, Size};

    use super::*;
    use crate::config::window_settings::WindowGeometry;

    #[test]
    fn settings_files_without_view_preferences_load_with_defaults() -> serde_json::Result<()> {
        let settings: Settings = serde_json::from_str(r#"{"recent_folders":["/diary"]}"#)?;

        assert_eq!(settings.recent_folders, vec!["/diary".to_string()]);
        assert_eq!(settings.diary_view_mode, DiaryViewMode::List);
        assert_eq!(settings.editor_mode, EditorMode::Edit);
        assert_eq!(settings.language, Language::System);
        Ok(())
    }

    #[test]
    fn window_and_layout_survive_a_round_trip() -> serde_json::Result<()> {
        let settings = Settings {
            window: WindowSettings {
                normal: Some(WindowGeometry::new(
                    Point::new(12.0, 34.0),
                    Size::new(1000.0, 700.0),
                )),
                maximized: true,
            },
            layout: LayoutSettings::from_ratios(0.42, 0.65),
            ..Settings::default()
        };

        let restored: Settings = serde_json::from_str(&serde_json::to_string_pretty(&settings)?)?;

        assert_eq!(restored.window, settings.window);
        assert_eq!(restored.layout, settings.layout);
        Ok(())
    }

    #[test]
    fn settings_without_window_or_layout_load_with_defaults() -> serde_json::Result<()> {
        let settings: Settings = serde_json::from_str("{}")?;

        assert_eq!(settings.window, WindowSettings::default());
        assert_eq!(settings.layout, LayoutSettings::default());
        Ok(())
    }

    #[test]
    fn the_snapshot_of_the_app_state_carries_window_and_layout() {
        let mut app_state = AppState::new();
        app_state.window.maximized = true;
        app_state.list_split_ratio = 0.4;
        app_state.editor_split_ratio = 0.6;

        let settings = Settings::from(&app_state);

        assert!(settings.window.maximized);
        assert_eq!(settings.layout.list_split_percent, 40.0);
        assert_eq!(settings.layout.editor_split_percent, 60.0);
    }

    #[test]
    fn view_preferences_survive_a_round_trip() -> serde_json::Result<()> {
        let settings = Settings {
            diary_view_mode: DiaryViewMode::Calendar,
            editor_mode: EditorMode::Split,
            language: Language::Turkish,
            ..Settings::default()
        };

        let restored: Settings = serde_json::from_str(&serde_json::to_string(&settings)?)?;

        assert_eq!(restored.diary_view_mode, DiaryViewMode::Calendar);
        assert_eq!(restored.editor_mode, EditorMode::Split);
        assert_eq!(restored.language, Language::Turkish);
        Ok(())
    }
}
