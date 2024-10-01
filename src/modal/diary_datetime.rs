use std::fmt::Display;

use chrono::{DateTime, Offset, TimeZone};
use druid::Data;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd)]
pub struct DiaryDateTime<Tz: TimeZone>(DateTime<Tz>);

impl<Tz: TimeZone + Ord> Ord for DiaryDateTime<Tz> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
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
