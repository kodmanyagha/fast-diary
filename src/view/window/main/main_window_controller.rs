use std::sync::Arc;

use druid::{widget::Controller, Env, Event, EventCtx, Selector, Widget};

use crate::modal::app_state::{AppState, DiaryListItem, OpenFilePurpose};

pub const DIARY_ADD_ITEM: Selector<DiaryListItem> = Selector::new("diary.add_item");
pub const DIARY_SET_CURRENT: Selector<DiaryListItem> = Selector::new("diary.set_current");
pub const DIARY_SAVE_CURRENT: Selector<()> = Selector::new("diary.save_current");

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
                let cmd_data = cmd.get_unchecked(DIARY_SET_CURRENT);

                app_state.current_diary = cmd_data.into();
                app_state.txt_diary = cmd_data.content.clone();

                pass_to_child = false;
            } else if cmd.is(DIARY_SAVE_CURRENT) {
                if app_state.current_diary.is_selected {
                    let diaries = Arc::make_mut(&mut app_state.diaries);

                    let found_diary = diaries
                        .iter_mut()
                        .find(|item| item.date.eq(&app_state.current_diary.diary.date));

                    if let Some(found_diary) = found_diary {
                        found_diary.content = app_state.txt_diary.clone();
                    }
                }
                pass_to_child = false;
            } else if cmd.is(druid::commands::OPEN_FILE) {
                let cmd_data = cmd.get_unchecked(druid::commands::OPEN_FILE);
                log::info!(">>> Open file result: {:?}", cmd_data);

                match app_state.open_file_purpose {
                    OpenFilePurpose::DiaryPath => {
                        if let Some(path) = cmd_data.path.to_str() {
                            app_state.selected_path = Some(path.to_string());
                        }
                    }
                }
            }
        }

        if pass_to_child {
            child.event(ctx, event, app_state, env)
        }
    }
}
