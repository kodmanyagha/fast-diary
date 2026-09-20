use druid::{widget::Flex, Widget, WidgetExt};
use druid_material_icons::normal::action::{LOGOUT, SETTINGS};

use crate::{
    consts::druid_selector,
    modal::{app_state::AppState, state::app_pages::AppPages},
    view::{
        page::diary::widgets::btn_create_widget::build_ui::build_btn_create,
        widget::{icon_button::icon_button, mode_selector::diary_view_mode_button},
    },
};

const BUTTON_SPACING: f64 = 4.0;

fn build_btn_settings() -> impl Widget<AppState> {
    icon_button(SETTINGS, "menu-file-settings", |_ctx, app_state, _env| {
        app_state.status_message.clear();
        app_state.page = AppPages::Settings;
    })
}

fn build_btn_close() -> impl Widget<AppState> {
    icon_button(LOGOUT, "menu-file-close-folder", |ctx, _app_state, _env| {
        ctx.submit_command(druid_selector::FOLDER_LOCK)
    })
}

/// Builds the row of buttons above the diaries: create a diary, open the settings and close the
/// folder on the left, and the button that switches between the list and the calendar on the
/// right.
pub fn build_icon_toolbar() -> impl Widget<AppState> {
    Flex::row()
        .with_child(build_btn_create())
        .with_spacer(BUTTON_SPACING)
        .with_child(build_btn_settings())
        .with_spacer(BUTTON_SPACING)
        .with_child(build_btn_close())
        .with_flex_spacer(1.0)
        .with_child(diary_view_mode_button())
        .expand_width()
}
