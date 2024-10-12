use std::fs::{self, DirEntry};

use chrono::{NaiveDate, NaiveDateTime, Utc};
use druid::{Data, Lens};

use crate::modal::{app_state_utils::diary_summary, diary_datetime::DiaryDate};

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
                    return Some(parsed_str.try_into().ok()?);
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

impl TryFrom<DirEntry> for DiaryListItem {
    type Error = String;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let mut item = DiaryListItem::new();
        item.set_file_name(
            value
                .file_name()
                .to_str()
                .ok_or("Filename can't readed".to_string())?
                .to_string(),
        );

        let filename = item.file_name.clone();
        let filename = filename
            .split(".")
            .next()
            .ok_or("Filename doesn't have any extension")?;

        item.set_date(
            DiaryListItem::parse_date(filename.into()).ok_or("Filename format is wrong.")?,
        );

        let original_content = fs::read_to_string(value.path())
            .map_err(|err| format!("Error occured when reading file content: {:?}", err))?;

        item.set_summary(diary_summary(&original_content));

        Ok(item)
    }
}
