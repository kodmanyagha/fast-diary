use druid::Selector;

use crate::modal::state::diary_list_item::DiaryListItem;

pub const DIARY_ADD_ITEM: Selector<DiaryListItem> = Selector::new("diary.add_item");
pub const DIARY_SET_CURRENT: Selector<DiaryListItem> = Selector::new("diary.set_current");
pub const DIARY_SAVE_CURRENT: Selector<()> = Selector::new("diary.save_current");
pub const DIARY_LOAD_FOLDER: Selector<()> = Selector::new("diary.load_folder");
pub const CREATE_NEW_DIARY: Selector<()> = Selector::new("diary.create");
