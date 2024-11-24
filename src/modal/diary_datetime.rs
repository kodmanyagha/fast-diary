use std::fmt::Display;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};
use druid::Data;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd)]
pub struct DiaryDateTime<Tz: TimeZone>(DateTime<Tz>);

pub type DiaryDate = DiaryDateTime<Utc>;

impl<Tz: TimeZone> DiaryDateTime<Tz> {
    pub fn timestamp(&self) -> i64 {
        self.0.timestamp()
    }
}

impl<Tz: TimeZone> Display for DiaryDateTime<Tz> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.naive_utc().format("%Y-%m-%d %H:%M:%S"))
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
        let datetime = Utc.from_local_datetime(&value);

        match datetime {
            chrono::offset::LocalResult::Single(result) => Ok(DiaryDateTime(result)),
            _ => Err("Wrong NaiveDateTime".to_string()),
        }
    }
}

impl TryFrom<NaiveDate> for DiaryDateTime<Utc> {
    type Error = String;

    fn try_from(value: NaiveDate) -> Result<Self, Self::Error> {
        let datetime = Utc.from_local_datetime(&value.into());

        match datetime {
            chrono::offset::LocalResult::Single(result) => Ok(DiaryDateTime(result)),
            _ => Err("Wrong NaiveDateTime".to_string()),
        }
    }
}
