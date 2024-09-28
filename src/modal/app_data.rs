use std::fmt::Display;

use chrono::Utc;
use druid::{Data, Lens};
use im::HashMap;

use super::diary_datetime::DiaryDateTime;

#[derive(Clone, Data, Lens)]
pub struct AppData {
    pub app_title: String,
    pub page: AppPages,
    pub selected_path: Option<String>,
    pub encrypt_key: Option<String>,
    pub diaries: Option<HashMap<DiaryDateTime<Utc>, String>>,
    pub current_diary: Option<(DiaryDateTime<Utc>, String)>,
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
            current_diary: None,
            diaries: None,
            encrypt_key: None,
            selected_path: None,
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct DiaryNote {}
