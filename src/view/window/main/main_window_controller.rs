use std::time::{Duration, Instant};

use anyhow::{anyhow, Context};
use chrono::{Local, NaiveDate, Timelike};
use druid::{
    widget::Controller, Command, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, TimerToken, Widget,
};

use crate::{
    config::settings::Settings,
    consts::druid_selector,
    modal::{
        app_state::{AppState, OpenFilePurpose},
        app_state_utils::{diaries_compare_rev, diary_in_hour, entries_by_day},
        state::{
            calendar_month::CalendarMonth, current_diary::CurrentDiary,
            diary_list_item::DiaryListItem,
        },
    },
    storage::diary_store::DiaryStore,
    utils::{consts::DEFAULT_DIARY_NAME, diary::summarize},
    view::window::about::about_window::build_about_window,
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

    /// Creates a diary for the current moment and opens it. The current hour gets only one diary,
    /// so when it already has one, that diary is opened instead. The diary that was open is saved
    /// before.
    pub fn handle_diary_create(
        &mut self,
        _ctx: &mut EventCtx,
        app_state: &mut AppState,
    ) -> anyhow::Result<()> {
        let now = Local::now();
        let (today, hour) = (now.date_naive(), now.hour());
        let current_diary = &app_state.current_diary;
        let is_draft_of_this_hour_open = current_diary.is_selected
            && current_diary.is_draft
            && current_diary.diary.date.local_date() == today
            && current_diary.diary.date.local_hour() == hour;
        if is_draft_of_this_hour_open {
            return Ok(());
        }

        let diary_of_this_hour = match diary_in_hour(&app_state.diaries, today, hour).cloned() {
            Some(diary) => diary,
            None => self.create_diary_of_now(app_state)?,
        };

        self.show_diary(app_state, diary_of_this_hour)
    }

    /// Creates a diary that is named after the current moment and adds it to the diary list.
    fn create_diary_of_now(&mut self, app_state: &mut AppState) -> anyhow::Result<DiaryListItem> {
        let store = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("Base path didn't selected."))?;
        let diary_file_name = format!(
            "{}.{}",
            Local::now().format(DEFAULT_DIARY_NAME),
            store.file_extension()
        );
        tracing::info!("New diary file creating: {diary_file_name}");

        store
            .create(&diary_file_name)
            .with_context(|| format!("Could not create diary `{diary_file_name}`."))?;
        let diary = DiaryListItem::from_file_name(&diary_file_name, String::new())
            .map_err(|err| anyhow!("Could not list diary `{diary_file_name}`: {err}"))?;

        app_state.diaries.push_back(diary.clone());
        app_state.diaries.sort_by(diaries_compare_rev);

        Ok(diary)
    }

    /// Makes `diary` the opened diary and lets the calendar show it. The diary that was open is
    /// saved first, and nothing is read again when `diary` is the open one.
    fn show_diary(&mut self, app_state: &mut AppState, diary: DiaryListItem) -> anyhow::Result<()> {
        let current_diary = &app_state.current_diary;
        let is_already_open = current_diary.is_selected
            && !current_diary.is_draft
            && current_diary.diary.file_name == diary.file_name;

        if is_already_open {
            app_state.calendar_month = app_state
                .calendar_month
                .window_showing(CalendarMonth::containing(diary.date.local_date()));
            return Ok(());
        }

        if app_state.current_diary.is_selected {
            if let Err(err) = self.handle_diary_save_current(app_state) {
                tracing::warn!("Could not save the diary that was open: {err:#}");
            }
        }

        self.open_diary(app_state, diary)
    }

    pub fn set_diary_base_path(&mut self, app_state: &mut AppState, path: String) {
        app_state.diary_base_path = Some(path.clone());

        let mut settings = Settings::from(&*app_state);
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
        let diary = cmd.get_unchecked(druid_selector::DIARY_SET_CURRENT).clone();

        self.open_diary(app_state, diary)
    }

    fn open_diary(&mut self, app_state: &mut AppState, diary: DiaryListItem) -> anyhow::Result<()> {
        let selected_diary = CurrentDiary::from(diary);
        let file_name = selected_diary.diary.file_name.clone();

        let text = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("Please select a path."))?
            .read_text(&file_name)
            .with_context(|| format!("Error occured when reading diary `{file_name}`."))?;

        app_state.calendar_month =
            app_state
                .calendar_month
                .window_showing(CalendarMonth::containing(
                    selected_diary.diary.date.local_date(),
                ));
        app_state.current_diary = selected_diary;
        app_state.txt_diary = text;

        Ok(())
    }

    /// Opens the latest diary of `date`, or an empty draft at 00:00:00 of that day when nothing
    /// was written on it yet.
    fn handle_diary_start_draft(
        &mut self,
        app_state: &mut AppState,
        date: NaiveDate,
    ) -> anyhow::Result<()> {
        let month = CalendarMonth::containing(date);
        if let Some(day_entries) = entries_by_day(&app_state.diaries, month).remove(&date) {
            return self.open_diary(app_state, day_entries.latest);
        }

        let store = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("Please select a path."))?;
        let draft = DiaryListItem::for_day(date, store.file_extension())
            .map_err(|err| anyhow!("Could not start a diary on {date}: {err}"))?;

        app_state.calendar_month = app_state.calendar_month.window_showing(month);
        app_state.current_diary = CurrentDiary::draft(draft);
        app_state.txt_diary.clear();

        Ok(())
    }

    /// Creates the file of the opened draft and lists it, so that it is saved like any other
    /// diary from now on.
    fn promote_draft(
        &mut self,
        app_state: &mut AppState,
        store: &DiaryStore,
    ) -> anyhow::Result<()> {
        let draft = app_state.current_diary.diary.clone();

        store
            .create(&draft.file_name)
            .with_context(|| format!("Could not create diary `{}`.", draft.file_name))?;

        app_state.current_diary.is_draft = false;
        app_state.diaries.push_back(draft);
        app_state.diaries.sort_by(diaries_compare_rev);

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

        if app_state.current_diary.is_draft {
            if app_state.txt_diary.trim().is_empty() {
                return Ok(());
            }
            self.promote_draft(app_state, &store)?;
        }

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
        Self::persist_window_if_changed(ctx, app_state);

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

    /// Remembers the position, size and state of the window in the app state.
    fn capture_window(ctx: &EventCtx, app_state: &mut AppState) {
        let window = ctx.window();
        app_state.window = app_state.window.observe(
            window.get_window_state(),
            window.get_position(),
            window.get_size(),
        );
    }

    fn save_settings(app_state: &AppState) {
        if let Err(err) = Settings::from(app_state).save() {
            tracing::error!("Could not save settings: {err:#}");
        }
    }

    /// Saves the settings when the window was moved, resized or maximized since it was last
    /// looked at.
    fn persist_window_if_changed(ctx: &EventCtx, app_state: &mut AppState) {
        let remembered_window = app_state.window;
        Self::capture_window(ctx, app_state);

        if app_state.window != remembered_window {
            Self::save_settings(app_state);
        }
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
            Self::capture_window(ctx, app_state);
            Self::save_settings(app_state);
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
            } else if cmd.is(druid_selector::DIARY_START_DRAFT) {
                let date = *cmd.get_unchecked(druid_selector::DIARY_START_DRAFT);
                if let Err(err) = self.handle_diary_start_draft(app_state, date) {
                    tracing::error!("Error on DIARY_START_DRAFT: {err:#}");
                    app_state.status_message = format!("{err:#}");
                }
                pass_event_to_child = false;
            } else if cmd.is(druid_selector::DIARY_SAVE_CURRENT) {
                if let Err(err) = self.handle_diary_save_current(app_state) {
                    tracing::warn!("Could not save current diary: {err:#}");
                }

                pass_event_to_child = false;
            } else if cmd.is(druid_selector::APP_SHOW_ABOUT) {
                ctx.new_window(build_about_window());
                pass_event_to_child = false;
            } else if cmd.is(druid_selector::SETTINGS_SAVE) {
                Self::capture_window(ctx, app_state);
                Self::save_settings(app_state);
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
                if let Err(err) = self.handle_diary_create(ctx, app_state) {
                    tracing::error!("File create error: {err:#}");
                }
                pass_event_to_child = false;
            } else if cmd.is(druid_selector::EDITOR_CURSOR_LINE_CHANGED) {
                app_state.editor_cursor_line =
                    *cmd.get_unchecked(druid_selector::EDITOR_CURSOR_LINE_CHANGED);
                pass_event_to_child = false;
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
