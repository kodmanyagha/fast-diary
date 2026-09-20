use druid::{
    keyboard_types::Key, widget::Controller, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, Widget,
};

use crate::{
    consts::druid_selector,
    modal::{app_state::AppState, app_state_utils::relative_diary},
};

#[derive(Default)]
pub struct DiaryListController;

impl DiaryListController {
    pub fn new() -> Self {
        Self
    }

    fn select_offset(&self, ctx: &mut EventCtx, app_state: &AppState, offset: isize) {
        let Some(next_diary) = relative_diary(app_state, offset) else {
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
            if cmd.is(druid_selector::DIARY_LOAD_FOLDER)
                || cmd.is(druid_selector::DIARY_BROWSER_FOCUS)
            {
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

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &AppState,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            ctx.submit_command(druid_selector::DIARY_BROWSER_FOCUS);
        }

        child.lifecycle(ctx, event, data, env)
    }
}
