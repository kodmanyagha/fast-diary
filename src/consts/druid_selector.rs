use chrono::NaiveDate;
use druid::Selector;

use crate::modal::state::diary_list_item::DiaryListItem;

pub const DIARY_ADD_ITEM: Selector<DiaryListItem> = Selector::new("diary.add_item");
pub const DIARY_SET_CURRENT: Selector<DiaryListItem> = Selector::new("diary.set_current");
pub const DIARY_START_DRAFT: Selector<NaiveDate> = Selector::new("diary.start_draft");
pub const DIARY_SAVE_CURRENT: Selector<()> = Selector::new("diary.save_current");
pub const DIARY_LOAD_FOLDER: Selector<()> = Selector::new("diary.load_folder");
pub const CREATE_NEW_DIARY: Selector<()> = Selector::new("diary.create");
pub const DIARY_BROWSER_FOCUS: Selector<()> = Selector::new("diary.browser_focus");
pub const APP_SHOW_ABOUT: Selector<()> = Selector::new("app.show_about");
pub const SETTINGS_SAVE: Selector<()> = Selector::new("settings.save");
pub const FOLDER_OPEN: Selector<()> = Selector::new("folder.open");
pub const FOLDER_ENCRYPT: Selector<()> = Selector::new("folder.encrypt");
pub const FOLDER_LOCK: Selector<()> = Selector::new("folder.lock");
pub const FOLDER_CHANGE_PASSWORD: Selector<()> = Selector::new("folder.change_password");
pub const EDITOR_CURSOR_LINE_CHANGED: Selector<usize> = Selector::new("editor.cursor_line_changed");
