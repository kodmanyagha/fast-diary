use std::fmt::Display;

use druid::Data;

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
