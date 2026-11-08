use std::sync::Arc;

use druid::{Data, Lens};
use im::Vector;

use super::state::app_pages::AppPages;
use super::state::{
    calendar_month::CalendarMonth, current_diary::CurrentDiary, diary_list_item::DiaryListItem,
    diary_view_mode::DiaryViewMode, editor_mode::EditorMode, language::Language,
    pending_confirmation::PendingConfirmation,
};
use crate::{
    config::{layout_settings::LayoutSettings, window_settings::WindowSettings},
    storage::{
        asset_store::AssetStore, codec::Codec, diary_store::DiaryStore, history::HistoryPolicy,
    },
    vault::folder_key::FolderKey,
};

pub struct DiariesWithSelectionLens;

impl DiariesWithSelectionLens {
    fn combine(app_state: &AppState) -> Vector<(DiaryListItem, bool)> {
        let selected_file_name = app_state
            .current_diary
            .is_selected
            .then(|| app_state.current_diary.diary.file_name.clone());

        app_state
            .diaries
            .iter()
            .map(|item| {
                let is_selected = selected_file_name.as_deref() == Some(item.file_name.as_str());
                (item.clone(), is_selected)
            })
            .collect()
    }
}

impl Lens<AppState, Vector<(DiaryListItem, bool)>> for DiariesWithSelectionLens {
    fn with<V, F: FnOnce(&Vector<(DiaryListItem, bool)>) -> V>(&self, data: &AppState, f: F) -> V {
        f(&Self::combine(data))
    }

    fn with_mut<V, F: FnOnce(&mut Vector<(DiaryListItem, bool)>) -> V>(
        &self,
        data: &mut AppState,
        f: F,
    ) -> V {
        let mut combined = Self::combine(data);
        let result = f(&mut combined);
        data.diaries = combined.into_iter().map(|(item, _)| item).collect();
        result
    }
}

#[derive(Clone, PartialEq, Data)]
pub enum OpenFilePurpose {
    DiaryPath,
    ImageInsert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Data)]
pub enum FolderProtection {
    Plain,
    Encrypted,
}

#[derive(Debug, Clone, Default, Data, Lens)]
pub struct PasswordChangeForm {
    pub current_password: String,
    pub new_password: String,
    pub confirmation: String,
}

pub struct DiaryBasePathLens;

impl Lens<AppState, String> for DiaryBasePathLens {
    fn with<V, F: FnOnce(&String) -> V>(&self, data: &AppState, f: F) -> V {
        match &data.diary_base_path {
            Some(path) => f(path),
            None => f(&String::new()),
        }
    }

    fn with_mut<V, F: FnOnce(&mut String) -> V>(&self, data: &mut AppState, f: F) -> V {
        let mut current = data.diary_base_path.clone().unwrap_or_default();
        let result = f(&mut current);
        data.diary_base_path = (!current.is_empty()).then_some(current);
        result
    }
}

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub app_title: String,
    pub page: AppPages,
    pub password: String,
    pub password_confirmation: String,
    pub password_change: PasswordChangeForm,
    pub status_message: String,

    pub open_file_purpose: OpenFilePurpose,

    pub diary_base_path: Option<String>,
    pub folder_protection: FolderProtection,
    pub folder_key: Option<Arc<FolderKey>>,
    pub recent_folders: Vector<String>,
    pub pending_confirmation: Option<PendingConfirmation>,

    pub diaries: Vector<DiaryListItem>,
    pub diary_view_mode: DiaryViewMode,
    pub calendar_month: CalendarMonth,

    pub current_diary: CurrentDiary,
    pub txt_diary: String,
    pub editor_cursor_line: usize,
    pub editor_mode: EditorMode,
    pub language: Language,

    pub window: WindowSettings,
    pub list_split_ratio: f64,
    pub editor_split_ratio: f64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            app_title: "Fast Diary".to_string(),
            page: AppPages::Main,
            password: "".to_string(),
            password_confirmation: "".to_string(),
            password_change: PasswordChangeForm::default(),
            status_message: "".to_string(),
            folder_protection: FolderProtection::Plain,
            folder_key: None,
            open_file_purpose: OpenFilePurpose::DiaryPath,
            diary_base_path: None,
            recent_folders: Vector::new(),
            pending_confirmation: None,
            diaries: Vector::new(),
            diary_view_mode: DiaryViewMode::default(),
            calendar_month: CalendarMonth::current().shifted(-1),
            current_diary: CurrentDiary::new().with_is_selected(false),
            txt_diary: "".into(),
            editor_cursor_line: 0,
            editor_mode: EditorMode::default(),
            language: Language::default(),
            window: WindowSettings::default(),
            list_split_ratio: LayoutSettings::default().list_split_ratio(),
            editor_split_ratio: LayoutSettings::default().editor_split_ratio(),
        }
    }

    pub fn get_diary_base_path(&self) -> Option<String> {
        self.diary_base_path.clone()
    }

    /// Returns the codec of the selected diary folder. Returns `None` when no folder is
    /// selected or when the folder is encrypted but not unlocked, so that a locked folder can
    /// never be read or written as plain text by accident.
    fn folder_codec(&self) -> Option<Codec> {
        self.diary_base_path.as_ref()?;
        match (self.folder_protection, &self.folder_key) {
            (FolderProtection::Plain, _) => Some(Codec::Plain),
            (FolderProtection::Encrypted, Some(folder_key)) => {
                Some(Codec::Encrypted(folder_key.clone()))
            }
            (FolderProtection::Encrypted, None) => None,
        }
    }

    /// Returns the store of the selected diary folder, under the conditions of
    /// [`Self::folder_codec`].
    pub fn diary_store(&self) -> Option<DiaryStore> {
        let base_path = self.diary_base_path.as_ref()?;

        Some(DiaryStore::new(
            base_path,
            HistoryPolicy::default(),
            self.folder_codec()?,
        ))
    }

    /// Returns the store of the files that the diaries of the selected folder refer to, under
    /// the conditions of [`Self::folder_codec`].
    pub fn asset_store(&self) -> Option<AssetStore> {
        let base_path = self.diary_base_path.as_ref()?;

        Some(AssetStore::new(base_path, self.folder_codec()?))
    }

    /// Selects the folder that was opened last, when it still exists on the disk.
    pub fn select_last_recent_folder(&mut self) {
        self.diary_base_path = self
            .recent_folders
            .front()
            .filter(|folder_path| std::path::Path::new(folder_path).is_dir())
            .cloned();
    }

    /// Returns the selected folder when it is one of the recent folders.
    pub fn selected_recent_folder(&self) -> Option<&String> {
        self.diary_base_path
            .as_ref()
            .filter(|folder_path| self.recent_folders.contains(folder_path))
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_recent_folders(selected: Option<&str>, recent_folders: &[&str]) -> AppState {
        let mut app_state = AppState::new();
        app_state.diary_base_path = selected.map(str::to_string);
        app_state.recent_folders = recent_folders.iter().map(|path| path.to_string()).collect();
        app_state
    }

    #[test]
    fn the_last_opened_folder_is_selected_when_it_exists() -> anyhow::Result<()> {
        let last = tempfile::tempdir()?;
        let older = tempfile::tempdir()?;
        let last_path = last.path().to_string_lossy().into_owned();
        let older_path = older.path().to_string_lossy().into_owned();
        let mut app_state = state_with_recent_folders(None, &[&last_path, &older_path]);

        app_state.select_last_recent_folder();

        assert_eq!(app_state.diary_base_path, Some(last_path));
        Ok(())
    }

    #[test]
    fn a_last_folder_that_is_gone_or_missing_selects_nothing() -> anyhow::Result<()> {
        let parent = tempfile::tempdir()?;
        let gone = parent.path().join("gone").to_string_lossy().into_owned();
        let mut app_state = state_with_recent_folders(None, &[&gone]);

        app_state.select_last_recent_folder();
        assert_eq!(app_state.diary_base_path, None);

        let mut empty = state_with_recent_folders(None, &[]);
        empty.select_last_recent_folder();
        assert_eq!(empty.diary_base_path, None);
        Ok(())
    }

    #[test]
    fn a_locked_encrypted_folder_has_no_stores() {
        let mut app_state = state_with_recent_folders(Some("/diary"), &[]);
        app_state.folder_protection = FolderProtection::Encrypted;

        assert!(app_state.diary_store().is_none());
        assert!(app_state.asset_store().is_none());
    }

    #[test]
    fn a_plain_folder_has_both_stores_and_no_folder_has_none() {
        assert!(state_with_recent_folders(Some("/diary"), &[])
            .asset_store()
            .is_some());
        assert!(state_with_recent_folders(None, &[]).asset_store().is_none());
    }

    #[test]
    fn the_selected_folder_is_a_recent_folder_when_it_is_listed() {
        let app_state = state_with_recent_folders(Some("/b"), &["/a", "/b"]);

        assert_eq!(
            app_state.selected_recent_folder().map(String::as_str),
            Some("/b")
        );
    }

    #[test]
    fn an_unlisted_or_missing_selection_is_not_a_recent_folder() {
        assert_eq!(
            state_with_recent_folders(Some("/c"), &["/a"]).selected_recent_folder(),
            None
        );
        assert_eq!(
            state_with_recent_folders(None, &["/a"]).selected_recent_folder(),
            None
        );
    }
}
