use chrono::{DateTime, Offset, TimeZone};
use druid::Data;

#[derive(Clone, Debug)]
pub struct DiaryDateTime<Tz: TimeZone>(DateTime<Tz>);

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
