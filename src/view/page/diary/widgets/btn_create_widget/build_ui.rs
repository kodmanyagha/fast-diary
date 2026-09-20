use druid::Widget;
use druid_material_icons::normal::content::ADD;

use crate::{
    consts::druid_selector, modal::app_state::AppState, view::widget::icon_button::icon_button,
};

pub fn build_btn_create() -> impl Widget<AppState> {
    icon_button(ADD, "menu-file-new-diary", |ctx, _app_state, _env| {
        ctx.submit_command(druid_selector::CREATE_NEW_DIARY)
    })
}
