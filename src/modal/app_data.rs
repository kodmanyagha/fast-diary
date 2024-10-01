use std::{fmt::Display, sync::Arc};

use chrono::Utc;
use druid::{Data, Lens};
use im::OrdMap;

use super::diary_datetime::DiaryDateTime;

#[derive(Debug, Clone, Data, Lens)]
pub struct DiaryListItem {
    pub date: DiaryDateTime<Utc>,
    pub title: String,
}

impl DiaryListItem {
    pub fn new(date: DiaryDateTime<Utc>, title: String) -> Self {
        Self { date, title }
    }
}

#[derive(Clone, Data, Lens)]
pub struct AppData {
    pub app_title: String,
    pub page: AppPages,
    pub password: String,
    pub selected_path: Option<String>,
    pub encrypt_key: Option<String>,

    pub diaries: Arc<Vec<DiaryListItem>>,

    pub current_diary: Option<DiaryListItem>,
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

impl AppData {
    pub fn new() -> Self {
        Self {
            app_title: "Fast Diary".to_string(),
            page: AppPages::Main,
            password: "".to_string(),
            current_diary: None,
            encrypt_key: None,
            selected_path: None,
            diaries: Arc::new(vec![]),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct DiaryNote {}
