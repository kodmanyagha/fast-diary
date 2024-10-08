use std::{fmt::Display, sync::Arc};

use chrono::Utc;
use druid::{Data, Lens};

use super::diary_datetime::DiaryDateTime;

#[derive(Debug, Clone, Data, Lens)]
pub struct DiaryListItem {
    pub date: DiaryDateTime<Utc>,
    pub title: String,
    pub content: String,
}

impl DiaryListItem {
    pub fn new(date: DiaryDateTime<Utc>, title: String) -> Self {
        Self {
            date,
            title,
            content: "".into(),
        }
    }

    pub fn new_fresh() -> Self {
        DiaryListItem::new(Utc::now().into(), "".into())
    }

    pub fn new_content(date: DiaryDateTime<Utc>, title: String, content: String) -> Self {
        Self {
            date,
            title,
            content,
        }
    }
}

#[derive(Debug, Clone, Data, Lens)]
pub struct CurrentDiary {
    pub is_selected: bool,
    pub diary: DiaryListItem,
}

impl CurrentDiary {
    pub fn new() -> Self {
        Self {
            is_selected: false,
            diary: DiaryListItem::new_fresh(),
        }
    }
}

impl From<DiaryListItem> for CurrentDiary {
    fn from(value: DiaryListItem) -> Self {
        Self {
            is_selected: true,
            diary: value,
        }
    }
}

impl From<&DiaryListItem> for CurrentDiary {
    fn from(value: &DiaryListItem) -> Self {
        Self {
            is_selected: true,
            diary: value.clone(),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub app_title: String,
    pub page: AppPages,
    pub password: String,
    pub selected_path: Option<String>,
    pub encrypt_key: Option<String>,

    pub diaries: Arc<Vec<DiaryListItem>>,

    pub current_diary: CurrentDiary,
    pub txt_diary: String,
}

#[derive(Clone, PartialEq, Debug)]
pub enum AppPages {
    Main,
    Diary,
    Settings,
}

impl Display for AppPages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Data for AppPages {
    fn same(&self, other: &Self) -> bool {
        self.eq(other)
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            app_title: "Fast Diary".to_string(),
            page: AppPages::Main,
            password: "".to_string(),
            encrypt_key: None,
            selected_path: None,
            diaries: Arc::new(vec![]),
            current_diary: CurrentDiary::new(),
            txt_diary: "".into(),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct DiaryNote {}
