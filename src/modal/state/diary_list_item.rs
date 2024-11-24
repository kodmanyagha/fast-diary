use std::fs::{self, DirEntry};

use chrono::{NaiveDate, Utc};
use druid::{Data, Lens};

use crate::{
    give,
    modal::{app_state_utils::diary_summary, diary_datetime::DiaryDate},
};

#[derive(Debug, Clone, Data, Lens)]
pub struct DiaryListItem {
    pub date: DiaryDate,
    pub summary: String,
    pub file_name: String,
}

impl DiaryListItem {
    pub fn new() -> Self {
        Self {
            date: Utc::now().into(),
            summary: "".into(),
            file_name: "".into(),
        }
    }

    pub fn with_date(mut self, date: DiaryDate) -> Self {
        self.date = date;
        self
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }

    pub fn with_file_name(mut self, file_name: String) -> Self {
        self.file_name = file_name;
        self
    }

    pub fn set_date(&mut self, date: DiaryDate) {
        self.date = date;
    }

    pub fn set_summary(&mut self, summary: String) {
        self.summary = summary;
    }

    pub fn set_file_name(&mut self, file_name: String) {
        self.file_name = file_name;
    }

    fn parse_date(date_str: String) -> Option<DiaryDate> {
        let formats = vec![
            "%Y%m%d%H%M%S",
            "%Y%m%d",
            "%y%m%d",
            "%y%M%d",
            "%Y%m%d%H%M",
            "%y%m%d%H%M",
            "%Y_%m_%d_%H_%M_%S",
            "%Y_%m_%d_%H_%M",
            "%y_%m_%d_%H_%M_%S",
            "%y_%m_%d_%H_%M",
        ];

        for format in formats {
            let parsed_str = NaiveDate::parse_from_str(&date_str, format);
            //let parsed_str = NaiveDateTime::parse_from_str(&date_str, format);

            match parsed_str {
                Ok(parsed_str) => {
                    // log::info!(">>> Correct format: {}", format);
                    return parsed_str.try_into().ok();
                }
                Err(_err) => {
                    // log::warn!(
                    //     ">>> Wrong format. Date: {}, format: {}, err: {:?}",
                    //     date_str,
                    //     format,
                    //     _err
                    // );
                }
            }
        }

        None
    }
}

impl Default for DiaryListItem {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<DirEntry> for DiaryListItem {
    type Error = String;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let mut diary_list_item = DiaryListItem::new();
        let filename = give!(value.file_name().to_str()).to_string();

        diary_list_item.set_file_name(filename);

        let filename = filename
            .split(".")
            .next()
            .ok_or("Filename doesn't have any extension")?;

        diary_list_item.set_date(
            DiaryListItem::parse_date(filename.into()).ok_or("Filename format is wrong.")?,
        );

        diary_list_item.set_summary(diary_summary(value.path()));

        Ok(diary_list_item)
    }
}
