use druid::{widget::Controller, Event, Selector, Widget, WidgetId};

use crate::modal::app_state::AppState;

pub const BTN_CREATE_CLICK: Selector<()> = Selector::new("btn_create.click");

pub struct BtnCreateController {
    widget_id: WidgetId,
}

impl BtnCreateController {
    pub fn new(widget_id: WidgetId) -> Self {
        BtnCreateController { widget_id }
    }
}

impl<W: Widget<AppState>> Controller<AppState, W> for BtnCreateController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut AppState,
        env: &druid::Env,
    ) {
        if let Event::Command(cmd) = event {
            if cmd.is(BTN_CREATE_CLICK) {
                log::info!(
                    "BTN_CREATE_CLICK cmd triggered, widget id: {:?}",
                    self.widget_id
                );
            }
        }

        child.event(ctx, event, data, env)
    }
}
