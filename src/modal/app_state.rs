use druid::{Data, Lens};
use im::Vector;

use super::state::app_pages::AppPages;
use super::state::{current_diary::CurrentDiary, diary_list_item::DiaryListItem};

/// Pairs each diary with whether it is the currently selected one, so list
/// item widgets can render a different background without needing access to
/// the rest of `AppState`.
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

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub app_title: String,
    pub page: AppPages,
    pub password: String,

    pub open_file_purpose: OpenFilePurpose,

    pub diary_base_path: Option<String>,
    pub encrypt_key: Option<String>,

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
            encrypt_key: None,
            open_file_purpose: OpenFilePurpose::DiaryPath,
            diary_base_path: None,
            diaries: Vector::new(),
            current_diary: CurrentDiary::new().with_is_selected(false),
            txt_diary: "".into(),
        }
    }

    pub fn get_diary_base_path(&self) -> Option<String> {
        self.diary_base_path.clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
