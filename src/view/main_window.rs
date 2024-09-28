use druid::{
    widget::{Controller, ViewSwitcher},
    Widget, WidgetExt,
};

use crate::modal::app_data::{AppData, AppPages};

use super::pages;

#[derive(Debug, Default)]
struct MainWindowController {
    //
}

impl MainWindowController {
    pub fn new() -> Self {
        Self {
            ..MainWindowController::default()
        }
    }
}

impl<W: Widget<AppData>> Controller<AppData, W> for MainWindowController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut AppData,
        env: &druid::Env,
    ) {
        log::info!("An event occured: {:?}", event);

        child.event(ctx, event, data, env)
    }
}

pub fn build_ui() -> impl Widget<AppData> {
    let page_switcher = ViewSwitcher::new(
        |data: &AppData, _env| data.page.clone(),
        |selector, _data, _env| match selector {
            AppPages::Main => Box::new(pages::main::build_ui()),
            AppPages::Diary => Box::new(pages::diary::build_ui()),
            AppPages::Settings => Box::new(pages::settings::build_ui()),
        },
    )
    .controller(MainWindowController::new());

    page_switcher
}
