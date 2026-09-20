use chrono::{NaiveDate, NaiveDateTime, Utc};
use druid::{Data, Lens};

use crate::modal::diary_datetime::DiaryDate;

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

    /// Builds a list item from a diary file name such as `240101123000.md.enc`, whose leading
    /// part holds the diary date.
    pub fn from_file_name(file_name: &str, summary: String) -> Result<Self, String> {
        let date_part = file_name
            .split('.')
            .next()
            .ok_or("Filename doesn't have any extension")?;
        let date = Self::parse_date(date_part.to_string()).ok_or("Filename format is wrong.")?;

        Ok(Self::new()
            .with_date(date)
            .with_summary(summary)
            .with_file_name(file_name.to_string()))
    }

    fn parse_date(date_str: String) -> Option<DiaryDate> {
        let formats = vec![
            "%y%m%d%H%M%S",
            "%Y%m%d%H%M%S",
            "%Y%m%d",
            "%y%m%d",
            "%y%M%d",
            "%y%M%D",
            "%Y%M%D",
            "%Y%m%d%H%M",
            "%y%m%d%H%M",
            "%Y_%m_%d_%H_%M_%S",
            "%Y_%m_%d_%H_%M",
            "%y_%m_%d_%H_%M_%S",
            "%y_%m_%d_%H_%M",
        ];

        for format in formats {
            if let Ok(parse_result) = NaiveDateTime::parse_from_str(&date_str, format) {
                return parse_result.try_into().ok();
            }
            if let Ok(parse_result) = NaiveDate::parse_from_str(&date_str, format) {
                return parse_result.try_into().ok();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_encrypted_file_names() {
        let plain = DiaryListItem::from_file_name("240101123000.md", "s".into());
        let encrypted = DiaryListItem::from_file_name("240101123000.md.enc", "s".into());

        assert!(plain.is_ok());
        assert!(encrypted.is_ok());
        assert_eq!(
            plain.map(|item| item.date.timestamp()),
            encrypted.map(|item| item.date.timestamp())
        );
    }

    #[test]
    fn keeps_file_name_and_summary() -> Result<(), String> {
        let item = DiaryListItem::from_file_name("240101123000.md.enc", "hello".into())?;

        assert_eq!(item.file_name, "240101123000.md.enc");
        assert_eq!(item.summary, "hello");
        Ok(())
    }

    #[test]
    fn rejects_names_without_a_date() {
        assert!(DiaryListItem::from_file_name("notes.md", String::new()).is_err());
        assert!(DiaryListItem::from_file_name(".encrypted", String::new()).is_err());
    }
}
