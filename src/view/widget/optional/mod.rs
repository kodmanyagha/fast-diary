use chrono::DateTime;
use chrono::Utc;
use druid::widget::CrossAxisAlignment;
use druid::widget::Flex;
use druid::widget::FlexParams;
use druid::Command;
use druid::Modifiers;
use druid::Target;
use druid::{
    widget::{Controller, Label, TextBox, ViewSwitcher},
    Event, EventCtx, Widget, WidgetExt,
};

use crate::modal::app_state::AppState;
use crate::view::window::main::main_window_controller::DIARY_SAVE_CURRENT;

#[derive(Debug, Default)]
struct DiaryTextController {
    last_save_time: DateTime<Utc>,
}

impl DiaryTextController {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl<W: Widget<String>> Controller<String, W> for DiaryTextController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &druid::Env,
    ) {
        match event {
            Event::KeyUp(key) => {
                if key.code.eq(&druid::Code::KeyS) && key.mods.contains(Modifiers::CONTROL) {
                    ctx.submit_command(Command::new(DIARY_SAVE_CURRENT, (), Target::Global));
                }
            }
            Event::Command(cmd) => {
                log::info!(">>> TextBoxController Event::Command {:?}", cmd);
            }
            _ => {}
        }

        child.event(ctx, event, data, env)
    }
}

pub fn optional() -> impl Widget<AppState> {
    ViewSwitcher::new(
        |data: &AppState, env| {
            // log::info!(">>> view switcher picker executed");
            data.current_diary.is_selected
        },
        |selector, data, env| {
            // log::info!(">>> view switcher builder executed, {:?}", selector);
            if *selector {
                Box::new(
                    Flex::column()
                        .with_flex_child(
                            Flex::column()
                                .with_flex_child(
                                    Label::dynamic(|data: &AppState, _| {
                                        data.current_diary.diary.date.clone().to_string()
                                    }),
                                    FlexParams::new(20_f64, CrossAxisAlignment::Center),
                                )
                                .expand_width()
                                .expand_height(),
                            FlexParams::new(10_f64, CrossAxisAlignment::Center),
                        )
                        .with_flex_child(
                            TextBox::multiline()
                                .with_line_wrapping(true)
                                .expand_width()
                                .expand_height()
                                .controller(DiaryTextController::new())
                                .lens(AppState::txt_diary),
                            FlexParams::new(90_f64, CrossAxisAlignment::Start),
                        )
                        .expand_width()
                        .expand_height(),
                )
            } else {
                Box::new(Label::new("Please select a diary").padding(5.0))
            }
        },
    )
}
