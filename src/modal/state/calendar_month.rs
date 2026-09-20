use chrono::{Datelike, Days, Local, NaiveDate};
use druid::Data;

pub const DAYS_IN_WEEK: usize = 7;
pub const CALENDAR_ROWS: usize = 6;
pub const CALENDAR_CELLS: usize = DAYS_IN_WEEK * CALENDAR_ROWS;
pub const MONTHS_SHOWN: usize = 3;

/// A calendar month whose days are laid out in weeks that start on Monday. The calendar shows
/// [`MONTHS_SHOWN`] months in a row, and the month that the app state keeps is the first of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Data)]
pub struct CalendarMonth {
    year: i32,
    month: u32,
}

impl CalendarMonth {
    pub fn containing(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }

    pub fn current() -> Self {
        Self::containing(Local::now().date_naive())
    }

    /// Returns the month that lies `months` after this one, or before it when negative.
    pub fn shifted(&self, months: i32) -> Self {
        let month_index = self.year * 12 + self.month as i32 - 1 + months;

        Self {
            year: month_index.div_euclid(12),
            month: month_index.rem_euclid(12) as u32 + 1,
        }
    }

    /// Returns the months that the calendar shows when this month is its first one.
    pub fn window(&self) -> [CalendarMonth; MONTHS_SHOWN] {
        std::array::from_fn(|index| self.shifted(index as i32))
    }

    pub fn shows(&self, month: CalendarMonth) -> bool {
        self.window().contains(&month)
    }

    /// Returns the first month of a calendar that shows `month`: this calendar when it already
    /// does, otherwise one that has `month` in the middle.
    pub fn window_showing(&self, month: CalendarMonth) -> Self {
        if self.shows(month) {
            *self
        } else {
            month.shifted(-1)
        }
    }

    /// Returns the month counted from 1 for January.
    pub fn month_number(&self) -> u32 {
        self.month
    }

    /// Returns the title of the month, such as `September 2026`, with `month_name` as the name of
    /// the month in the wanted language.
    pub fn title(&self, month_name: &str) -> String {
        format!("{month_name} {}", self.year)
    }

    /// Returns the title of the whole calendar, such as `September - November 2026`, with the
    /// names of its first and its last month in the wanted language.
    pub fn window_title(&self, first_month_name: &str, last_month_name: &str) -> String {
        let last_month = self.shifted(MONTHS_SHOWN as i32 - 1);

        if self.year == last_month.year {
            format!("{first_month_name} - {last_month_name} {}", last_month.year)
        } else {
            format!(
                "{first_month_name} {} - {last_month_name} {}",
                self.year, last_month.year
            )
        }
    }

    /// Returns the number of weeks that the days of the month are spread over.
    pub fn week_rows(&self) -> usize {
        let blank_cells_before = self.first_day().map_or(0, |first_day| {
            first_day.weekday().num_days_from_monday() as usize
        });

        (blank_cells_before + self.days_in_month()).div_ceil(DAYS_IN_WEEK)
    }

    /// Returns the number of weeks of each of the months that the calendar shows.
    pub fn window_rows(&self) -> [usize; MONTHS_SHOWN] {
        self.window().map(|month| month.week_rows())
    }

    /// Returns the date shown in the grid cell at `cell_index`, or `None` for the blank cells
    /// before the first and after the last day of the month.
    pub fn date_at_cell(&self, cell_index: usize) -> Option<NaiveDate> {
        let first_day = self.first_day()?;
        let blank_cells_before = first_day.weekday().num_days_from_monday() as usize;
        let day_offset = cell_index.checked_sub(blank_cells_before)?;

        first_day
            .checked_add_days(Days::new(day_offset as u64))
            .filter(|date| date.month() == self.month)
    }

    fn first_day(&self) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year, self.month, 1)
    }

    fn days_in_month(&self) -> usize {
        let next_month = self.shifted(1);

        self.first_day()
            .zip(next_month.first_day())
            .map_or(0, |(first_day, next_first_day)| {
                (next_first_day - first_day).num_days() as usize
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month_of(year: i32, month: u32) -> Option<CalendarMonth> {
        NaiveDate::from_ymd_opt(year, month, 1).map(CalendarMonth::containing)
    }

    #[test]
    fn month_starting_on_monday_has_no_leading_blank_cells() {
        let january_2024 = month_of(2024, 1);

        assert_eq!(
            january_2024.and_then(|month| month.date_at_cell(0)),
            NaiveDate::from_ymd_opt(2024, 1, 1)
        );
    }

    #[test]
    fn leading_and_trailing_cells_are_blank() {
        let february_2024 = month_of(2024, 2);

        assert_eq!(february_2024.and_then(|month| month.date_at_cell(2)), None);
        assert_eq!(
            february_2024.and_then(|month| month.date_at_cell(3)),
            NaiveDate::from_ymd_opt(2024, 2, 1)
        );
        assert_eq!(
            february_2024.and_then(|month| month.date_at_cell(31)),
            NaiveDate::from_ymd_opt(2024, 2, 29)
        );
        assert_eq!(february_2024.and_then(|month| month.date_at_cell(32)), None);
    }

    #[test]
    fn every_day_of_the_month_appears_exactly_once() {
        let december_2025 = month_of(2025, 12);

        let days_in_grid = (0..CALENDAR_CELLS)
            .filter_map(|cell| december_2025.and_then(|month| month.date_at_cell(cell)))
            .count();

        assert_eq!(days_in_grid, 31);
    }

    #[test]
    fn shifting_crosses_year_boundaries_in_both_directions() {
        let december_2024 = month_of(2024, 12);

        assert_eq!(
            december_2024.map(|month| month.shifted(1)),
            month_of(2025, 1)
        );
        assert_eq!(
            december_2024.map(|month| month.shifted(3)),
            month_of(2025, 3)
        );
        assert_eq!(
            december_2024.map(|month| month.shifted(-12)),
            month_of(2023, 12)
        );
        assert_eq!(
            month_of(2025, 1).map(|month| month.shifted(-1)),
            december_2024
        );
        assert_eq!(
            month_of(2025, 2).map(|month| month.shifted(-3)),
            month_of(2024, 11)
        );
    }

    #[test]
    fn the_calendar_shows_three_consecutive_months() {
        let window = month_of(2024, 11).map(|month| month.window().map(Some));

        assert_eq!(
            window,
            Some([month_of(2024, 11), month_of(2024, 12), month_of(2025, 1)])
        );
    }

    #[test]
    fn a_calendar_keeps_its_months_while_they_show_the_wanted_one() {
        let calendar = month_of(2024, 5);
        let wanted_months = [month_of(2024, 5), month_of(2024, 6), month_of(2024, 7)];

        wanted_months.iter().for_each(|wanted| {
            assert_eq!(
                calendar
                    .zip(*wanted)
                    .map(|(calendar, wanted)| calendar.window_showing(wanted)),
                calendar
            );
        });
    }

    #[test]
    fn a_calendar_that_does_not_show_the_wanted_month_puts_it_in_the_middle() {
        let shifted_calendar = month_of(2024, 5)
            .zip(month_of(2024, 9))
            .map(|(calendar, month)| calendar.window_showing(month));

        assert_eq!(shifted_calendar, month_of(2024, 8));
    }

    #[test]
    fn the_number_of_weeks_follows_the_days_and_the_first_weekday() {
        assert_eq!(month_of(2021, 2).map(|month| month.week_rows()), Some(4));
        assert_eq!(month_of(2024, 1).map(|month| month.week_rows()), Some(5));
        assert_eq!(month_of(2024, 12).map(|month| month.week_rows()), Some(6));
        assert_eq!(
            month_of(2024, 12).map(|month| month.window_rows()),
            Some([6, 5, 5])
        );
    }

    #[test]
    fn title_shows_the_given_month_name_and_the_year() {
        assert_eq!(
            month_of(2026, 9).map(|month| month.title("Eylül")),
            Some("Eylül 2026".to_string())
        );
    }

    #[test]
    fn the_title_of_the_calendar_names_its_first_and_last_month() {
        assert_eq!(
            month_of(2026, 9).map(|month| month.window_title("Eylül", "Kasım")),
            Some("Eylül - Kasım 2026".to_string())
        );
        assert_eq!(
            month_of(2026, 11).map(|month| month.window_title("Kasım", "Ocak")),
            Some("Kasım 2026 - Ocak 2027".to_string())
        );
    }

    #[test]
    fn the_month_number_counts_from_one() {
        assert_eq!(month_of(2026, 9).map(|month| month.month_number()), Some(9));
    }
}
