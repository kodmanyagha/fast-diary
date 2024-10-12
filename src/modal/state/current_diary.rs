use druid::{Data, Lens};

use super::diary_list_item::DiaryListItem;

#[derive(Debug, Clone, Data, Lens)]
pub struct CurrentDiary {
    pub is_selected: bool,
    pub diary: DiaryListItem,
}

impl CurrentDiary {
    pub fn new() -> Self {
        Self {
            is_selected: false,
            diary: DiaryListItem::new(),
        }
    }

    pub fn with_is_selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }
}

impl From<DiaryListItem> for CurrentDiary {
    fn from(value: DiaryListItem) -> Self {
        Self {
            is_selected: true,
            diary: value,
        }
    }
}

impl From<&DiaryListItem> for CurrentDiary {
    fn from(value: &DiaryListItem) -> Self {
        Self {
            is_selected: true,
            diary: value.clone(),
        }
    }
}
