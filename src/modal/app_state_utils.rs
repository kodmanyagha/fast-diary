use std::cmp::Ordering;

use super::state::diary_list_item::DiaryListItem;

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

pub fn diary_summary(diary: &str) -> String {
    let result = if diary.chars().count() > 30 {
        let x = diary.chars();
        let x = x.take(30);
        let x: String = x.take(30).collect();

        format!("{}...", x.trim())
    } else {
        diary.trim().to_string()
    };

    result
        .replace("\r", "")
        .replace("\n", " ")
        .trim()
        .to_string()
}
