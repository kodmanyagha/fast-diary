use std::sync::Arc;

use druid::{widget::Button, Widget, WidgetExt, WidgetId};

use crate::{consts::druid_selector, modal::app_state::AppState};

pub fn build_btn_create() -> impl Widget<AppState> {
    let arc_widget_id = Arc::new(WidgetId::next());

    Button::new("Create")
        .on_click(move |ctx, _data, _env| {
            ctx.submit_command(druid_selector::CREATE_NEW_DIARY);
        })
        .with_id(*arc_widget_id)
        .expand()
}
