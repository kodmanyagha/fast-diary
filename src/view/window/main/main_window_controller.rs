use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
    sync::Arc,
};

use anyhow::anyhow;
use chrono::Local;
use druid::{widget::Controller, Command, Env, Event, EventCtx, Selector, Widget};

use crate::{
    giver,
    modal::{
        app_state::{app_state_derived_lenses::diaries, AppState, OpenFilePurpose},
        app_state_utils::diaries_compare_rev,
        state::diary_list_item::DiaryListItem,
    },
    utils::{consts::DEFAULT_DIARY_NAME, diary::diary_summary},
};

pub const DIARY_ADD_ITEM: Selector<DiaryListItem> = Selector::new("diary.add_item");
pub const DIARY_SET_CURRENT: Selector<DiaryListItem> = Selector::new("diary.set_current");
pub const DIARY_SAVE_CURRENT: Selector<()> = Selector::new("diary.save_current");
pub const DIARY_LOAD_FOLDER: Selector<()> = Selector::new("diary.load_folder");
pub const CREATE_NEW_DIARY: Selector<()> = Selector::new("diary.create");

#[derive(Debug, Default)]
pub struct MainWindowController;

impl MainWindowController {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_diary_create(
        &self,
        ctx: &mut EventCtx,
        app_state: &mut AppState,
    ) -> anyhow::Result<()> {
        let diary_file_name = format!("{}.md", Local::now().format(DEFAULT_DIARY_NAME));
        log::info!(">> Diary name: {diary_file_name}");

        let file_exist = app_state.diaries.iter().find(|item| {
            item.file_name
                .eq(format!("{}.md", diary_file_name).as_str())
        });

        if file_exist.is_some() {
            return Err(anyhow::anyhow!("Same diary already exist."));
        }

        let diary_base_path = app_state
            .diary_base_path
            .clone()
            .ok_or(anyhow::anyhow!("Base path didn't selected."))?;

        let _ = File::create_new(Path::new(&diary_base_path).join(&diary_file_name))?;

        Ok(())
    }

    pub fn load_folder(&mut self, ctx: &mut EventCtx, app_state: &mut AppState) -> Option<()> {
        let dir_content = fs::read_dir(app_state.diary_base_path.clone()?).ok()?;

        app_state.diaries.clear();

        dir_content.for_each(|item| {
            let file_dir_entry = giver!(item);
            let diary_list_item = giver!(DiaryListItem::try_from(file_dir_entry));
            app_state.diaries.push_back(diary_list_item);
        });
        app_state.diaries.sort_by(diaries_compare_rev);

        Some(())
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

        let fullpath_str = format!(
            "{}/{}",
            app_state
                .diary_base_path
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

    fn handle_diary_save_current(
        &mut self,
        cmd: &Command,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
    ) -> anyhow::Result<()> {
        if !app_state.current_diary.is_selected {
            return Err(anyhow!("Diary not selected."));
        }

        let selected_path = app_state
            .get_diary_base_path()
            .ok_or(anyhow!("Diary path did not selected."))?;

        let found_diary = app_state
            .diaries
            .iter_mut()
            .find(|item| item.date.eq(&app_state.current_diary.diary.date));

        let current_file = Path::new(&selected_path).join(&app_state.current_diary.diary.file_name);

        if let Some(found_diary) = found_diary {
            found_diary.summary = diary_summary(current_file.clone())?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(current_file)?;

        file.write_all(app_state.txt_diary.as_bytes())?;
        file.sync_all()?;

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
        let mut pass_event_to_child = true;

        if let Event::WindowSize(_size) = event {
            //log::info!("Window resize event: {:?}", _size);
        } else if let Event::MouseMove(_mouse_event) = event {
            // log::info!("Mouse event: {:?}", _mouse_event.window_pos);
        } else if let Event::Command(cmd) = event {
            // TODO Improve this logic, there are lots of if-else blocks. Optimize this.

            if cmd.is(DIARY_ADD_ITEM) {
                let cmd_data = cmd.get_unchecked(DIARY_ADD_ITEM);
                app_state.diaries.push_back(cmd_data.to_owned());
            } else if cmd.is(DIARY_SET_CURRENT) {
                let result = self.handle_diary_set_current(cmd, ctx, event, app_state);

                if let Err(err) = result {
                    log::error!("Error on DIARY_SET_CURRENT: {}", err);
                }

                pass_event_to_child = false;
            } else if cmd.is(DIARY_SAVE_CURRENT) {
                let _ = self.handle_diary_save_current(cmd, ctx, event, app_state);

                pass_event_to_child = false;
            } else if cmd.is(druid::commands::OPEN_FILE) {
                let cmd_data = cmd.get_unchecked(druid::commands::OPEN_FILE);

                match app_state.open_file_purpose {
                    OpenFilePurpose::DiaryPath => {
                        if let Some(path) = cmd_data.path.to_str() {
                            app_state.diary_base_path = Some(path.to_string());
                        }
                    }
                }
            } else if cmd.is(DIARY_LOAD_FOLDER) {
                self.load_folder(ctx, app_state);
            } else if cmd.is(CREATE_NEW_DIARY) {
                let create_result = self.handle_diary_create(ctx, app_state);

                if let Err(err) = create_result {
                    log::error!("File create error: {err}");
                }

                self.load_folder(ctx, app_state);
            }
        }

        if pass_event_to_child {
            child.event(ctx, event, app_state, env)
        }
    }
}
