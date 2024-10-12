use std::cmp::Ordering;

use super::state::diary_list_item::DiaryListItem;

pub fn diaries_compare(item1: &DiaryListItem, item2: &DiaryListItem) -> Ordering {
    if item1.date.timestamp() > item2.date.timestamp() {
        Ordering::Greater
    } else if item1.date.timestamp() < item2.date.timestamp() {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

pub fn diaries_compare_rev(item1: &DiaryListItem, item2: &DiaryListItem) -> Ordering {
    if item1.date.timestamp() > item2.date.timestamp() {
        Ordering::Less
    } else if item1.date.timestamp() < item2.date.timestamp() {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

pub fn diary_summary(diary: &str) -> String {
    let result = if diary.len() > 30 {
        format!("{}...", diary[0..30].trim().to_string())
    } else {
        diary.trim().to_string()
    };

    result.replace("\n", " ").trim().to_string()
}
