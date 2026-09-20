use std::sync::Arc;

use druid::{Data, Lens};
use im::Vector;

use super::state::app_pages::AppPages;
use super::state::{current_diary::CurrentDiary, diary_list_item::DiaryListItem};
use crate::{
    storage::{codec::Codec, diary_store::DiaryStore, history::HistoryPolicy},
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

    pub diaries: Vector<DiaryListItem>,

    pub current_diary: CurrentDiary,
    pub txt_diary: String,
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
            diaries: Vector::new(),
            current_diary: CurrentDiary::new().with_is_selected(false),
            txt_diary: "".into(),
        }
    }

    pub fn get_diary_base_path(&self) -> Option<String> {
        self.diary_base_path.clone()
    }

    /// Returns the store of the selected diary folder. Returns `None` when no folder is
    /// selected or when the folder is encrypted but not unlocked, so that a locked folder can
    /// never be read or written as plain text by accident.
    pub fn diary_store(&self) -> Option<DiaryStore> {
        let base_path = self.diary_base_path.as_ref()?;
        let codec = match (self.folder_protection, &self.folder_key) {
            (FolderProtection::Plain, _) => Codec::Plain,
            (FolderProtection::Encrypted, Some(folder_key)) => Codec::Encrypted(folder_key.clone()),
            (FolderProtection::Encrypted, None) => return None,
        };

        Some(DiaryStore::new(base_path, HistoryPolicy::default(), codec))
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
