use druid::Selector;

use crate::modal::state::diary_list_item::DiaryListItem;

pub const DIARY_ADD_ITEM: Selector<DiaryListItem> = Selector::new("diary.add_item");
pub const DIARY_SET_CURRENT: Selector<DiaryListItem> = Selector::new("diary.set_current");
pub const DIARY_SAVE_CURRENT: Selector<()> = Selector::new("diary.save_current");
pub const DIARY_LOAD_FOLDER: Selector<()> = Selector::new("diary.load_folder");
pub const CREATE_NEW_DIARY: Selector<()> = Selector::new("diary.create");
pub const FOLDER_OPEN: Selector<()> = Selector::new("folder.open");
pub const FOLDER_ENCRYPT: Selector<()> = Selector::new("folder.encrypt");
pub const FOLDER_LOCK: Selector<()> = Selector::new("folder.lock");
pub const FOLDER_CHANGE_PASSWORD: Selector<()> = Selector::new("folder.change_password");
