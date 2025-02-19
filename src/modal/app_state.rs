use druid::{Data, Lens};
use im::Vector;

use super::state::app_pages::AppPages;
use super::state::{current_diary::CurrentDiary, diary_list_item::DiaryListItem};

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
