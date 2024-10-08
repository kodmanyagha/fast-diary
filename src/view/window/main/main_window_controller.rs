use std::sync::Arc;

use druid::{widget::Controller, Env, Event, EventCtx, Selector, Widget};

use crate::modal::app_state::{AppState, DiaryListItem};

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
        app_data: &mut AppState,
        env: &Env,
    ) {
        if let Event::WindowSize(_size) = event {
            //log::info!("Window resize event: {:?}", _size);
        } else if let Event::MouseMove(_mouse_event) = event {
            // log::info!("Mouse event: {:?}", _mouse_event.window_pos);
        } else if let Event::Command(cmd) = event {
            if cmd.is(DIARY_ADD_ITEM) {
                let cmd_data = cmd.get_unchecked(DIARY_ADD_ITEM);

                let diaries = Arc::make_mut(&mut app_data.diaries);
                diaries.push(cmd_data.to_owned());
            } else if cmd.is(DIARY_SET_CURRENT) {
                let cmd_data = cmd.get_unchecked(DIARY_SET_CURRENT);

                app_data.current_diary = cmd_data.into();
                app_data.txt_diary = cmd_data.content.clone();
            } else if cmd.is(DIARY_SAVE_CURRENT) {
                if app_data.current_diary.is_selected {
                    let diaries = Arc::make_mut(&mut app_data.diaries);

                    let found_diary = diaries
                        .iter_mut()
                        .find(|item| item.date.eq(&app_data.current_diary.diary.date));

                    if let Some(found_diary) = found_diary {
                        found_diary.content = app_data.txt_diary.clone();
                    }
                }
            }
        }

        child.event(ctx, event, app_data, env)
    }
}
