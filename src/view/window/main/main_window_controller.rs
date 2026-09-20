use std::time::{Duration, Instant};

use anyhow::{anyhow, Context};
use chrono::Local;
use druid::{
    widget::Controller, Command, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, TimerToken, Widget,
};

use crate::{
    config::settings::Settings,
    consts::druid_selector,
    modal::{
        app_state::{AppState, OpenFilePurpose},
        state::current_diary::CurrentDiary,
    },
    utils::{consts::DEFAULT_DIARY_NAME, diary::summarize},
};

use super::main_window_controller_utils::{
    change_folder_password, encrypt_selected_folder, load_diary_items, lock_folder,
    open_selected_folder, refresh_folder_protection,
};

const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(5);
const AUTO_LOCK_AFTER: Duration = Duration::from_secs(10 * 60);

#[derive(Debug)]
pub struct MainWindowController {
    observed_base_path: Option<String>,
    autosave_timer: TimerToken,
    last_activity: Instant,
}

impl Default for MainWindowController {
    fn default() -> Self {
        Self::new()
    }
}

impl MainWindowController {
    pub fn new() -> Self {
        Self {
            observed_base_path: None,
            autosave_timer: TimerToken::INVALID,
            last_activity: Instant::now(),
        }
    }

    pub fn handle_diary_create(
        &self,
        _ctx: &mut EventCtx,
        app_state: &mut AppState,
    ) -> anyhow::Result<()> {
        let current_date = Local::now();
        if let Some(first_diary) = app_state.diaries.get(0) {
            let diff = current_date.timestamp() - first_diary.date.timestamp();
            tracing::info!(
                ?current_date,
                ?first_diary,
                ?diff,
                ">>>>>>>>> first_diary datetime"
            );

            if diff < 60 {
                return Err(anyhow::anyhow!("Wait a little"));
            }
        }

        let store = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("Base path didn't selected."))?;
        let diary_file_name = format!(
            "{}.{}",
            Local::now().format(DEFAULT_DIARY_NAME),
            store.file_extension()
        );
        tracing::info!("New diary file creating: {diary_file_name}");

        let file_exist = app_state
            .diaries
            .iter()
            .any(|item| item.file_name == diary_file_name);

        if file_exist {
            return Err(anyhow::anyhow!("Same diary already exist."));
        }

        store
            .create(&diary_file_name)
            .with_context(|| format!("Could not create diary `{diary_file_name}`."))
    }

    pub fn set_diary_base_path(&mut self, app_state: &mut AppState, path: String) {
        app_state.diary_base_path = Some(path.clone());

        let mut settings = Settings {
            recent_folders: app_state.recent_folders.iter().cloned().collect(),
        };
        settings.push_recent_folder(path);
        app_state.recent_folders = settings.recent_folders.iter().cloned().collect();

        if let Err(err) = settings.save() {
            tracing::error!("Could not save settings: {err}");
        }
    }

    pub fn load_folder(&mut self, _ctx: &mut EventCtx, app_state: &mut AppState) -> Option<()> {
        let store = app_state.diary_store()?;

        app_state.diaries = load_diary_items(&store)
            .inspect_err(|err| tracing::error!("Could not load diaries: {err:#}"))
            .ok()?;

        Some(())
    }

    pub fn handle_diary_set_current(
        &mut self,
        cmd: &Command,
        _ctx: &mut EventCtx,
        _event: &Event,
        app_state: &mut AppState,
    ) -> anyhow::Result<()> {
        let selected_diary =
            CurrentDiary::from(cmd.get_unchecked(druid_selector::DIARY_SET_CURRENT));
        let file_name = selected_diary.diary.file_name.clone();

        let text = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("Please select a path."))?
            .read_text(&file_name)
            .with_context(|| format!("Error occured when reading diary `{file_name}`."))?;

        app_state.current_diary = selected_diary;
        app_state.txt_diary = text;

        Ok(())
    }

    fn handle_diary_save_current(&mut self, app_state: &mut AppState) -> anyhow::Result<()> {
        if !app_state.current_diary.is_selected {
            return Err(anyhow!("Diary not selected."));
        }

        let store = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("Diary path did not selected."))?;
        let file_name = app_state.current_diary.diary.file_name.clone();

        store
            .write_text(&file_name, &app_state.txt_diary)
            .with_context(|| format!("Could not save diary `{file_name}`."))?;

        let updated_summary = summarize(&app_state.txt_diary);
        let found_diary = app_state
            .diaries
            .iter_mut()
            .find(|item| item.date.eq(&app_state.current_diary.diary.date));

        if let Some(found_diary) = found_diary {
            found_diary.summary = updated_summary;
        }

        Ok(())
    }

    /// Saves the opened diary, then locks the folder. The folder stays open when saving fails,
    /// so no unsaved text is lost.
    fn save_then_lock(&mut self, app_state: &mut AppState) -> anyhow::Result<()> {
        if app_state.current_diary.is_selected {
            self.handle_diary_save_current(app_state)?;
        }

        lock_folder(app_state);
        Ok(())
    }

    /// Saves the opened diary and locks the folder after a period without user activity.
    fn handle_autosave_tick(&mut self, ctx: &mut EventCtx, app_state: &mut AppState) {
        if app_state.current_diary.is_selected {
            if let Err(err) = self.handle_diary_save_current(app_state) {
                tracing::warn!("Autosave failed: {err:#}");
            }
        }

        let should_auto_lock =
            app_state.folder_key.is_some() && self.last_activity.elapsed() >= AUTO_LOCK_AFTER;
        if should_auto_lock {
            if let Err(err) = self.save_then_lock(app_state) {
                tracing::warn!("Automatic lock failed: {err:#}");
            }
        }

        self.autosave_timer = ctx.request_timer(AUTOSAVE_INTERVAL);
    }

    /// Resets the folder related state when the selected folder has changed.
    fn sync_folder_protection(&mut self, app_state: &mut AppState) {
        if self.observed_base_path != app_state.diary_base_path {
            self.observed_base_path = app_state.diary_base_path.clone();
            refresh_folder_protection(app_state);
        }
    }

    /// Shows the outcome of a folder command to the user.
    fn report_outcome(
        app_state: &mut AppState,
        outcome: anyhow::Result<()>,
        success_message: &str,
    ) {
        app_state.status_message = match outcome {
            Ok(()) => success_message.to_string(),
            Err(err) => format!("{err:#}"),
        };
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

        if matches!(event, Event::KeyDown(_) | Event::MouseDown(_)) {
            self.last_activity = Instant::now();
        }

        if let Event::WindowSize(_size) = event {
        } else if let Event::WindowDisconnected = event {
            if app_state.current_diary.is_selected {
                if let Err(err) = self.handle_diary_save_current(app_state) {
                    tracing::error!("Could not save current diary on window close: {err}");
                }
            }
        } else if let Event::Timer(token) = event {
            if *token == self.autosave_timer {
                self.handle_autosave_tick(ctx, app_state);
                pass_event_to_child = false;
            }
        } else if let Event::MouseMove(_mouse_event) = event {
        } else if let Event::Command(cmd) = event {
            if cmd.is(druid_selector::DIARY_ADD_ITEM) {
                let cmd_data = cmd.get_unchecked(druid_selector::DIARY_ADD_ITEM);
                app_state.diaries.push_back(cmd_data.to_owned());
            } else if cmd.is(druid_selector::DIARY_SET_CURRENT) {
                let result = self.handle_diary_set_current(cmd, ctx, event, app_state);

                if let Err(err) = result {
                    tracing::error!("Error on DIARY_SET_CURRENT: {}", err);
                }
                pass_event_to_child = false;
            } else if cmd.is(druid_selector::DIARY_SAVE_CURRENT) {
                if let Err(err) = self.handle_diary_save_current(app_state) {
                    tracing::warn!("Could not save current diary: {err:#}");
                }

                pass_event_to_child = false;
            } else if cmd.is(druid_selector::FOLDER_OPEN) {
                let outcome = open_selected_folder(app_state);
                Self::report_outcome(app_state, outcome, "");
                pass_event_to_child = false;
            } else if cmd.is(druid_selector::FOLDER_ENCRYPT) {
                let outcome = encrypt_selected_folder(app_state);
                Self::report_outcome(app_state, outcome, "");
                pass_event_to_child = false;
            } else if cmd.is(druid_selector::FOLDER_LOCK) {
                let outcome = self.save_then_lock(app_state);
                Self::report_outcome(app_state, outcome, "");
                pass_event_to_child = false;
            } else if cmd.is(druid_selector::FOLDER_CHANGE_PASSWORD) {
                let outcome = change_folder_password(app_state);
                Self::report_outcome(app_state, outcome, "Password changed.");
                pass_event_to_child = false;
            } else if cmd.is(druid::commands::OPEN_FILE) {
                let cmd_data = cmd.get_unchecked(druid::commands::OPEN_FILE);

                match app_state.open_file_purpose {
                    OpenFilePurpose::DiaryPath => {
                        if let Some(path) = cmd_data.path.to_str() {
                            self.set_diary_base_path(app_state, path.to_string());
                        }
                    }
                }
            } else if cmd.is(druid_selector::DIARY_LOAD_FOLDER) {
                self.load_folder(ctx, app_state);
            } else if cmd.is(druid_selector::CREATE_NEW_DIARY) {
                let create_result = self.handle_diary_create(ctx, app_state);
                if let Err(err) = create_result {
                    tracing::error!("File create error: {err}");
                }

                self.load_folder(ctx, app_state);
            }
        }

        if pass_event_to_child {
            child.event(ctx, event, app_state, env)
        }

        self.sync_folder_protection(app_state);
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        app_state: &AppState,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.autosave_timer = ctx.request_timer(AUTOSAVE_INTERVAL);
        }

        child.lifecycle(ctx, event, app_state, env)
    }
}
