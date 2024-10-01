use std::sync::Arc;

use druid::{
    widget::{Controller, ViewSwitcher},
    Color, Env, Event, EventCtx, Widget, WidgetExt,
};

use crate::{
    modal::{
        app_data::{AppData, AppPages},
        selector::ADD_DIARY_ITEM,
    },
    view::page,
};

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
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppData,
        env: &Env,
    ) {
        // log::info!("An event occured: {:?}", event);

        if let Event::WindowSize(size) = event {
            //log::info!("Window resize event: {:?}", size);
        } else if let Event::MouseMove(mouse_event) = event {
            // log::info!("Mouse event: {:?}", mouse_event.window_pos);
        } else if let Event::Command(cmd) = event {
            if cmd.is(ADD_DIARY_ITEM) {
                let cmd_data = cmd.get_unchecked(ADD_DIARY_ITEM);
                log::info!("ADD_DIARY_ITEM: {:?}", cmd_data);

                let mut diaries = data.diaries.as_ref().clone();
                diaries.push(cmd_data.to_owned());
                data.diaries = Arc::new(diaries);
            }
        }

        child.event(ctx, event, data, env)
    }
}

pub fn build_ui() -> impl Widget<AppData> {
    let page_switcher = ViewSwitcher::new(
        |data: &AppData, _env| data.page.clone(),
        |selector, _data, _env| match selector {
            AppPages::Main => Box::new(page::main::build_ui()),
            AppPages::Diary => Box::new(page::diary::build_ui()),
            AppPages::Settings => Box::new(page::settings::build_ui()),
        },
    );

    page_switcher
        .expand_width()
        .expand_height()
        .border(Color::rgb8(255, 0, 0), 0f64)
        .controller(MainWindowController::new())
}
