use std::sync::Arc;

use druid::{widget::Button, Widget, WidgetExt, WidgetId};

use crate::{
    modal::app_state::AppState, view::window::main::main_window_controller::CREATE_NEW_DIARY,
};

pub fn build_btn_create() -> impl Widget<AppState> {
    let arc_widget_id = Arc::new(WidgetId::next());

    // let widget_id_on_click_clone = arc_widget_id.clone();

    Button::new("Create")
        .on_click(move |ctx, data, env| {
            // ctx.submit_command(Command::new(
            //     BTN_CREATE_CLICK,
            //     (),
            //     *widget_id_on_click_clone,
            // ));

            ctx.submit_command(CREATE_NEW_DIARY);
        })
        .with_id(*arc_widget_id)
        // .controller(BtnCreateController::new(*arc_widget_id))
        .expand()
}
