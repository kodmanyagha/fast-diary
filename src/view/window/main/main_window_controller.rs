use std::{
    fs::{self},
    path::Path,
    sync::Arc,
};

use anyhow::anyhow;
use druid::{widget::Controller, Command, Env, Event, EventCtx, Selector, Widget};

use crate::modal::{
    app_state::{AppState, OpenFilePurpose},
    app_state_utils::{diaries_compare_rev, diary_summary},
    state::diary_list_item::DiaryListItem,
};

pub const DIARY_ADD_ITEM: Selector<DiaryListItem> = Selector::new("diary.add_item");
pub const DIARY_SET_CURRENT: Selector<DiaryListItem> = Selector::new("diary.set_current");
pub const DIARY_SAVE_CURRENT: Selector<()> = Selector::new("diary.save_current");
pub const DIARY_LOAD_FOLDER: Selector<()> = Selector::new("diary.load_folder");

#[derive(Debug, Default)]
pub struct MainWindowController {
    //
}

impl MainWindowController {
    pub fn new() -> Self {
        Self {
            ..MainWindowController::default()
        }
    }

    pub fn load_folder(&mut self, ctx: &mut EventCtx, app_state: &mut AppState) {
        let _: Option<()> = (|| {
            let dir_content = fs::read_dir(app_state.selected_path.clone()?).ok()?;
            let diaries_mut = Arc::make_mut(&mut app_state.diaries);
            diaries_mut.clear();

            dir_content.for_each(|item| {
                let _: anyhow::Result<()> = (|| {
                    let file_dir_entry = item.map_err(|_| anyhow!("Can't read item."))?;

                    diaries_mut.push(
                        file_dir_entry
                            .try_into()
                            .map_err(|err| anyhow!("{}:{} {}", file!(), line!(), err))?,
                    );

                    Ok(())
                })();
            });

            diaries_mut.sort_by(diaries_compare_rev);

            Some(())
        })();
    }

    pub fn handle_diary_set_current(
        &mut self,
        cmd: &Command,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
    ) -> anyhow::Result<()> {
        let cmd_data = cmd.get_unchecked(DIARY_SET_CURRENT);

        app_state.current_diary = cmd_data.into();

        // FIXME Think about the fullpath separator, windows using "\", others using "/"
        let fullpath_str = format!(
            "{}/{}",
            app_state
                .selected_path
                .clone()
                .ok_or(anyhow!("Please select a path."))?,
            app_state.current_diary.diary.file_name
        );
        let fullpath = Path::new(&fullpath_str);

        let original_content = fs::read_to_string(fullpath)
            .map_err(|err| anyhow!("Error occured when reading file content: {:?}", err))?;

        app_state.txt_diary = original_content;

        Ok(())
    }
}

impl<W: Widget<AppState>> Controller<AppState, W> for MainWindowController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
        env: &Env,
    ) {
        let mut pass_to_child = true;
        if let Event::WindowSize(_size) = event {
            //log::info!("Window resize event: {:?}", _size);
        } else if let Event::MouseMove(_mouse_event) = event {
            // log::info!("Mouse event: {:?}", _mouse_event.window_pos);
        } else if let Event::Command(cmd) = event {
            if cmd.is(DIARY_ADD_ITEM) {
                let cmd_data = cmd.get_unchecked(DIARY_ADD_ITEM);

                let diaries = Arc::make_mut(&mut app_state.diaries);
                diaries.push(cmd_data.to_owned());
            } else if cmd.is(DIARY_SET_CURRENT) {
                let result = self.handle_diary_set_current(cmd, ctx, event, app_state);

                match result {
                    Ok(_) => pass_to_child = false,
                    Err(err) => {
                        log::error!("An error occured while handling DIARY_SET_CURRENT: {}", err);
                    }
                }
            } else if cmd.is(DIARY_SAVE_CURRENT) {
                if app_state.current_diary.is_selected {
                    let diaries = Arc::make_mut(&mut app_state.diaries);

                    let found_diary = diaries
                        .iter_mut()
                        .find(|item| item.date.eq(&app_state.current_diary.diary.date));

                    if let Some(found_diary) = found_diary {
                        found_diary.summary = diary_summary(&app_state.txt_diary.clone());
                    }
                }
                pass_to_child = false;
            } else if cmd.is(druid::commands::OPEN_FILE) {
                let cmd_data = cmd.get_unchecked(druid::commands::OPEN_FILE);

                match app_state.open_file_purpose {
                    OpenFilePurpose::DiaryPath => {
                        if let Some(path) = cmd_data.path.to_str() {
                            app_state.selected_path = Some(path.to_string());
                        }
                    }
                }
            } else if cmd.is(DIARY_LOAD_FOLDER) {
                self.load_folder(ctx, app_state);
            }
        }

        if pass_to_child {
            child.event(ctx, event, app_state, env)
        }
    }
}
