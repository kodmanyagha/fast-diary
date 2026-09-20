use std::fmt::Display;

use chrono::{
    DateTime, Datelike, Local, NaiveDate, NaiveDateTime, Offset, TimeZone, Timelike, Utc,
};
use druid::Data;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd)]
pub struct DiaryDateTime<Tz: TimeZone>(DateTime<Tz>);

pub type DiaryDate = DiaryDateTime<Utc>;

impl<Tz: TimeZone> DiaryDateTime<Tz> {
    pub fn timestamp(&self) -> i64 {
        self.0.timestamp()
    }

    /// Returns the calendar day of this moment in the local time zone, which is the zone diary
    /// file names are written in.
    pub fn local_date(&self) -> NaiveDate {
        self.0.with_timezone(&Local).date_naive()
    }

    /// Returns the hour of the local time, from 0 to 23.
    pub fn local_hour(&self) -> u32 {
        self.0.with_timezone(&Local).hour()
    }

    /// Returns the month of the local date, counted from 1 for January.
    pub fn local_month(&self) -> u32 {
        self.local_date().month()
    }

    /// Formats the local date and time for reading, such as `15 October 2026 12:13:14`, with
    /// `month_name` as the name of the month in the wanted language.
    pub fn human_readable(&self, month_name: &str) -> String {
        let local = self.0.with_timezone(&Local);

        format!(
            "{} {month_name} {} {}",
            local.day(),
            local.year(),
            local.format("%H:%M:%S")
        )
    }
}

impl<Tz: TimeZone> Display for DiaryDateTime<Tz> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S")
        )
    }
}

impl<Tz: TimeZone + 'static> Data for DiaryDateTime<Tz> {
    fn same(&self, other: &Self) -> bool {
        self.0.timestamp() == other.0.timestamp()
            && self.0.offset().fix().eq(&other.0.offset().fix())
    }
}

impl<Tz: TimeZone> From<DateTime<Tz>> for DiaryDateTime<Tz> {
    fn from(value: DateTime<Tz>) -> Self {
        Self(value)
    }
}

impl TryFrom<NaiveDateTime> for DiaryDateTime<Utc> {
    type Error = String;

    fn try_from(value: NaiveDateTime) -> Result<Self, Self::Error> {
        let datetime = Local.from_local_datetime(&value);

        match datetime {
            chrono::offset::LocalResult::Single(result) => {
                Ok(DiaryDateTime(result.with_timezone(&Utc)))
            }
            _ => Err("Wrong NaiveDateTime".to_string()),
        }
    }
}

impl TryFrom<NaiveDate> for DiaryDateTime<Utc> {
    type Error = String;

    fn try_from(value: NaiveDate) -> Result<Self, Self::Error> {
        let datetime = Local.from_local_datetime(&value.into());

        match datetime {
            chrono::offset::LocalResult::Single(result) => {
                Ok(DiaryDateTime(result.with_timezone(&Utc)))
            }
            _ => Err("Wrong NaiveDateTime".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diary_date(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Option<DiaryDate> {
        NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|date| date.and_hms_opt(hour, minute, second))
            .and_then(|date_time| DiaryDate::try_from(date_time).ok())
    }

    #[test]
    fn dates_are_written_with_the_given_month_name_and_the_time() {
        assert_eq!(
            diary_date(2026, 10, 15, 12, 13, 14).map(|date| date.human_readable("Ekim")),
            Some("15 Ekim 2026 12:13:14".to_string())
        );
    }

    #[test]
    fn days_have_no_leading_zero_but_the_time_has() {
        assert_eq!(
            diary_date(2024, 1, 5, 0, 7, 9).map(|date| date.human_readable("January")),
            Some("5 January 2024 00:07:09".to_string())
        );
    }

    #[test]
    fn the_hour_follows_the_local_time() {
        assert_eq!(
            diary_date(2026, 10, 15, 23, 59, 59).map(|date| date.local_hour()),
            Some(23)
        );
        assert_eq!(
            diary_date(2026, 10, 15, 0, 0, 0).map(|date| date.local_hour()),
            Some(0)
        );
    }

    #[test]
    fn the_month_number_follows_the_local_date() {
        assert_eq!(
            diary_date(2026, 10, 15, 12, 13, 14).map(|date| date.local_month()),
            Some(10)
        );
    }
}
