use std::{cmp::Ordering, collections::HashMap};

use chrono::NaiveDate;
use im::Vector;

use super::{
    app_state::AppState,
    state::{calendar_month::CalendarMonth, diary_list_item::DiaryListItem},
};

pub fn diaries_compare(item1: &DiaryListItem, item2: &DiaryListItem) -> Ordering {
    item1.date.timestamp().cmp(&item2.date.timestamp())
}

pub fn diaries_compare_rev(item1: &DiaryListItem, item2: &DiaryListItem) -> Ordering {
    match item1.date.timestamp().cmp(&item2.date.timestamp()) {
        Ordering::Less => Ordering::Greater,
        Ordering::Equal => Ordering::Equal,
        Ordering::Greater => Ordering::Less,
    }
}

/// The diaries written on a single calendar day: the latest one and the older ones after it.
#[derive(Debug, Clone)]
pub struct DayEntries {
    pub latest: DiaryListItem,
    pub older: Vec<DiaryListItem>,
}

impl DayEntries {
    pub fn count(&self) -> usize {
        1 + self.older.len()
    }

    /// Returns the diary to open when the day is clicked. Clicking again goes on with the next
    /// older diary of the day and, after the oldest one, starts again with the latest one. A
    /// diary that is not of this day gives the latest one.
    pub fn diary_to_open(&self, opened_file_name: Option<&str>) -> &DiaryListItem {
        let newest_first = || std::iter::once(&self.latest).chain(self.older.iter());
        let opened_position = opened_file_name
            .and_then(|file_name| newest_first().position(|diary| diary.file_name == file_name));

        opened_position
            .and_then(|position| newest_first().nth((position + 1) % self.count()))
            .unwrap_or(&self.latest)
    }
}

/// Groups the diaries that were written in `month` by their local calendar day, each day with its
/// diaries ordered newest first.
pub fn entries_by_day(
    diaries: &Vector<DiaryListItem>,
    month: CalendarMonth,
) -> HashMap<NaiveDate, DayEntries> {
    diaries
        .iter()
        .filter(|diary| CalendarMonth::containing(diary.date.local_date()) == month)
        .fold(
            HashMap::<NaiveDate, Vec<&DiaryListItem>>::new(),
            |mut days, diary| {
                days.entry(diary.date.local_date()).or_default().push(diary);
                days
            },
        )
        .into_iter()
        .filter_map(|(date, mut day_diaries)| {
            day_diaries.sort_by_key(|diary| std::cmp::Reverse(diary.date.timestamp()));
            let (latest, older) = day_diaries.split_first()?;

            Some((
                date,
                DayEntries {
                    latest: (*latest).clone(),
                    older: older.iter().map(|diary| (*diary).clone()).collect(),
                },
            ))
        })
        .collect()
}

/// Returns the latest diary that was written in the local hour `hour` of `date`.
pub fn diary_in_hour(
    diaries: &Vector<DiaryListItem>,
    date: NaiveDate,
    hour: u32,
) -> Option<&DiaryListItem> {
    diaries
        .iter()
        .filter(|diary| diary.date.local_date() == date && diary.date.local_hour() == hour)
        .max_by_key(|diary| diary.date.timestamp())
}

/// Returns the diary that lies `offset` positions after the opened one in the diary list, which
/// is ordered newest first. Without an opened diary the newest one is returned.
pub fn relative_diary(app_state: &AppState, offset: isize) -> Option<DiaryListItem> {
    let current_diary = &app_state.current_diary;
    if current_diary.is_selected && current_diary.is_draft {
        return diary_next_to_moment(
            &app_state.diaries,
            current_diary.diary.date.timestamp(),
            offset,
        );
    }

    let opened_index = app_state
        .current_diary
        .is_selected
        .then(|| {
            app_state
                .diaries
                .iter()
                .position(|item| item.file_name == app_state.current_diary.diary.file_name)
        })
        .flatten();

    let target_index = match opened_index {
        Some(index) => index.checked_add_signed(offset)?,
        None => 0,
    };

    app_state.diaries.get(target_index).cloned()
}

/// Returns the diary that is `offset` positions older (positive) or newer (negative) than
/// `timestamp` in `diaries`, which are ordered newest first.
fn diary_next_to_moment(
    diaries: &Vector<DiaryListItem>,
    timestamp: i64,
    offset: isize,
) -> Option<DiaryListItem> {
    let skipped_diaries = offset.unsigned_abs().checked_sub(1)?;

    if offset > 0 {
        diaries
            .iter()
            .filter(|diary| diary.date.timestamp() < timestamp)
            .nth(skipped_diaries)
            .cloned()
    } else {
        diaries
            .iter()
            .rev()
            .filter(|diary| diary.date.timestamp() > timestamp)
            .nth(skipped_diaries)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modal::state::current_diary::CurrentDiary;

    fn diary(file_name: &str) -> DiaryListItem {
        DiaryListItem::from_file_name(file_name, String::new()).unwrap_or_default()
    }

    fn month_of(file_name: &str) -> CalendarMonth {
        CalendarMonth::containing(diary(file_name).date.local_date())
    }

    fn state_with(file_names: &[&str], opened: Option<&str>) -> AppState {
        let mut app_state = AppState::new();
        app_state.diaries = file_names.iter().map(|name| diary(name)).collect();
        if let Some(opened) = opened {
            app_state.current_diary = diary(opened).into();
        }
        app_state
    }

    #[test]
    fn keeps_the_diaries_of_each_day_newest_first() {
        let diaries: Vector<_> = [
            "240115090000.md",
            "240115200000.md",
            "240115120000.md",
            "240116100000.md",
            "240210100000.md",
        ]
        .iter()
        .map(|name| diary(name))
        .collect();

        let entries = entries_by_day(&diaries, month_of("240115090000.md"));

        assert_eq!(entries.len(), 2);
        let january_15th = NaiveDate::from_ymd_opt(2024, 1, 15).and_then(|day| entries.get(&day));
        assert_eq!(
            january_15th.map(|day| day.latest.file_name.as_str()),
            Some("240115200000.md")
        );
        assert_eq!(
            january_15th.map(|day| {
                day.older
                    .iter()
                    .map(|diary| diary.file_name.as_str())
                    .collect::<Vec<_>>()
            }),
            Some(vec!["240115120000.md", "240115090000.md"])
        );
        assert_eq!(january_15th.map(|day| day.count()), Some(3));
    }

    #[test]
    fn clicking_a_day_again_goes_through_its_diaries_and_starts_over() {
        let entries = DayEntries {
            latest: diary("240115200000.md"),
            older: vec![diary("240115120000.md"), diary("240115090000.md")],
        };
        let name_after = |opened: Option<&str>| entries.diary_to_open(opened).file_name.clone();

        assert_eq!(name_after(None), "240115200000.md");
        assert_eq!(name_after(Some("240115200000.md")), "240115120000.md");
        assert_eq!(name_after(Some("240115120000.md")), "240115090000.md");
        assert_eq!(name_after(Some("240115090000.md")), "240115200000.md");
        assert_eq!(name_after(Some("240116100000.md")), "240115200000.md");
    }

    #[test]
    fn a_day_with_one_diary_keeps_opening_that_diary() {
        let entries = DayEntries {
            latest: diary("240115200000.md"),
            older: Vec::new(),
        };

        assert_eq!(
            entries.diary_to_open(Some("240115200000.md")).file_name,
            "240115200000.md"
        );
    }

    #[test]
    fn finds_the_latest_diary_of_an_hour_of_a_day() {
        let diaries: Vector<_> = [
            "240115143000.md",
            "240115145900.md",
            "240115153000.md",
            "240116143000.md",
        ]
        .iter()
        .map(|name| diary(name))
        .collect();
        let day = NaiveDate::from_ymd_opt(2024, 1, 15);

        assert_eq!(
            day.and_then(|day| diary_in_hour(&diaries, day, 14))
                .map(|diary| diary.file_name.as_str()),
            Some("240115145900.md")
        );
        assert!(day
            .and_then(|day| diary_in_hour(&diaries, day, 13))
            .is_none());
    }

    #[test]
    fn ignores_diaries_of_other_months() {
        let diaries: Vector<_> = ["240210100000.md"].iter().map(|name| diary(name)).collect();

        assert!(entries_by_day(&diaries, month_of("240115090000.md")).is_empty());
    }

    #[test]
    fn relative_diary_moves_through_the_list() {
        let file_names = ["240103120000.md", "240102120000.md", "240101120000.md"];
        let app_state = state_with(&file_names, Some("240102120000.md"));

        assert_eq!(
            relative_diary(&app_state, 1).map(|item| item.file_name),
            Some("240101120000.md".to_string())
        );
        assert_eq!(
            relative_diary(&app_state, -1).map(|item| item.file_name),
            Some("240103120000.md".to_string())
        );
    }

    #[test]
    fn relative_diary_stops_at_both_ends() {
        let file_names = ["240103120000.md", "240102120000.md", "240101120000.md"];

        assert!(relative_diary(&state_with(&file_names, Some("240103120000.md")), -1).is_none());
        assert!(relative_diary(&state_with(&file_names, Some("240101120000.md")), 1).is_none());
    }

    #[test]
    fn relative_diary_of_a_draft_finds_its_neighbours_by_time() {
        let mut app_state = state_with(&["240103120000.md", "240101120000.md"], None);
        app_state.current_diary = CurrentDiary::draft(diary("240102000000.md"));

        assert_eq!(
            relative_diary(&app_state, 1).map(|item| item.file_name),
            Some("240101120000.md".to_string())
        );
        assert_eq!(
            relative_diary(&app_state, -1).map(|item| item.file_name),
            Some("240103120000.md".to_string())
        );
        assert!(relative_diary(&app_state, 2).is_none());
    }

    #[test]
    fn relative_diary_starts_at_the_newest_without_an_opened_diary() {
        let app_state = state_with(&["240103120000.md", "240102120000.md"], None);

        assert_eq!(
            relative_diary(&app_state, 1).map(|item| item.file_name),
            Some("240103120000.md".to_string())
        );
        assert!(relative_diary(&state_with(&[], None), 1).is_none());
    }
}
