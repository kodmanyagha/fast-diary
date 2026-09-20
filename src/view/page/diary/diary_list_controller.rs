use druid::{keyboard_types::Key, widget::Controller, Env, Event, EventCtx, Widget};

use crate::{consts::druid_selector, modal::app_state::AppState};

#[derive(Default)]
pub struct DiaryListController;

impl DiaryListController {
    pub fn new() -> Self {
        Self
    }

    fn select_offset(&self, ctx: &mut EventCtx, app_state: &mut AppState, offset: isize) {
        if app_state.diaries.is_empty() {
            return;
        }

        let current_index = app_state.current_diary.is_selected.then(|| {
            app_state
                .diaries
                .iter()
                .position(|item| item.file_name == app_state.current_diary.diary.file_name)
        });

        let next_index = match current_index.flatten() {
            Some(index) => {
                let new_index = index as isize + offset;
                if new_index < 0 || new_index as usize >= app_state.diaries.len() {
                    return;
                }
                new_index as usize
            }
            None => 0,
        };

        let Some(next_diary) = app_state.diaries.get(next_index).cloned() else {
            return;
        };

        ctx.submit_command(druid_selector::DIARY_SAVE_CURRENT.with(()));
        ctx.submit_command(druid_selector::DIARY_SET_CURRENT.with(next_diary));
    }
}

impl<W: Widget<AppState>> Controller<AppState, W> for DiaryListController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        if let Event::Command(cmd) = event {
            if cmd.is(druid_selector::DIARY_LOAD_FOLDER) {
                ctx.request_focus();
            }
        }

        if let Event::KeyDown(key_event) = event {
            match key_event.key {
                Key::ArrowUp => {
                    self.select_offset(ctx, data, -1);
                    ctx.set_handled();
                    return;
                }
                Key::ArrowDown => {
                    self.select_offset(ctx, data, 1);
                    ctx.set_handled();
                    return;
                }
                _ => {}
            }
        }

        child.event(ctx, event, data, env)
    }
}
