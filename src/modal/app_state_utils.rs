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
