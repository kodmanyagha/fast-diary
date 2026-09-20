use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

const SETTINGS_DIR_NAME: &str = "fast-diary";
const SETTINGS_FILE_NAME: &str = "settings.json";
const MAX_RECENT_FOLDERS: usize = 10;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub recent_folders: Vec<String>,
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
        fs::write(path, content)?;

        Ok(())
    }

    pub fn push_recent_folder(&mut self, folder_path: String) {
        self.recent_folders.retain(|item| item != &folder_path);
        self.recent_folders.insert(0, folder_path);
        self.recent_folders.truncate(MAX_RECENT_FOLDERS);
    }
}
