use druid::{widget::ViewSwitcher, Widget, WidgetExt};

use crate::{
    modal::{app_state::AppState, state::app_pages::AppPages},
    view::page::{diary::diary_page, main::main_page, settings::settings_page},
};

use super::main_window_controller::MainWindowController;

pub fn build_ui() -> impl Widget<AppState> {
    let page_switcher = ViewSwitcher::new(
        |data: &AppState, _env| data.page.clone(),
        |selector, _data, _env| match selector {
            AppPages::Main => Box::new(main_page::build_ui()),
            AppPages::Diary => Box::new(diary_page::build_ui()),
            AppPages::Settings => Box::new(settings_page::build_ui()),
        },
    );

    page_switcher
        .expand_width()
        .expand_height()
        .controller(MainWindowController::new())
}
