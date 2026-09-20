use druid::widget::CrossAxisAlignment;
use druid::widget::Flex;
use druid::widget::FlexParams;
use druid::Command;
use druid::Modifiers;
use druid::Target;
use druid::{
    theme,
    widget::{Controller, Label, TextBox, ViewSwitcher},
    Event, EventCtx, Widget, WidgetExt,
};

use crate::consts::druid_selector;
use crate::modal::app_state::AppState;
use crate::modal::state::editor_mode::EditorMode;
use crate::utils::localization::month_name;
use crate::view::widget::markdown_preview::markdown_preview;
use crate::view::widget::mode_selector::editor_mode_button;
use crate::view::widget::persistent_split::PersistentSplit;

const EDITOR_MIN_WIDTH: f64 = 120.0;
const PREVIEW_MIN_WIDTH: f64 = 120.0;
const TITLE_SPACING: f64 = 6.0;

#[derive(Debug, Default)]
struct DiaryTextController;

impl DiaryTextController {
    pub fn new() -> Self {
        Self
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
                    ctx.submit_command(Command::new(
                        druid_selector::DIARY_SAVE_CURRENT,
                        (),
                        Target::Global,
                    ));
                }
            }
            Event::Command(cmd) => {
                tracing::info!(">>> TextBoxController Event::Command {:?}", cmd);
            }
            _ => {}
        }

        child.event(ctx, event, data, env)
    }
}

fn build_text_editor() -> impl Widget<AppState> {
    TextBox::multiline()
        .with_line_wrapping(true)
        .expand_width()
        .expand_height()
        .controller(DiaryTextController::new())
        .lens(AppState::txt_diary)
}

fn build_editor_area(mode: EditorMode) -> Box<dyn Widget<AppState>> {
    match mode {
        EditorMode::Edit => Box::new(build_text_editor()),
        EditorMode::Preview => Box::new(markdown_preview()),
        EditorMode::Split => Box::new(
            PersistentSplit::columns(
                build_text_editor(),
                markdown_preview(),
                AppState::editor_split_ratio,
            )
            .bar_size(3f64)
            .min_widths(EDITOR_MIN_WIDTH, PREVIEW_MIN_WIDTH),
        ),
    }
}

fn build_title() -> impl Widget<AppState> {
    Label::dynamic(|data: &AppState, _env| {
        let date = &data.current_diary.diary.date;
        date.human_readable(&month_name(data.language, date.local_month()))
    })
    .with_font(theme::UI_FONT_BOLD)
}

fn build_title_row() -> impl Widget<AppState> {
    Flex::row()
        .with_child(build_title())
        .with_flex_spacer(1.0)
        .with_child(editor_mode_button().lens(AppState::editor_mode))
        .expand_width()
}

pub fn optional() -> impl Widget<AppState> {
    ViewSwitcher::new(
        |data: &AppState, _env| data.current_diary.is_selected,
        |selector, _data, _env| {
            if *selector {
                Box::new(
                    Flex::column()
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .with_child(build_title_row())
                        .with_spacer(TITLE_SPACING)
                        .with_flex_child(
                            ViewSwitcher::new(
                                |data: &AppState, _env| data.editor_mode,
                                |mode, _data, _env| build_editor_area(*mode),
                            ),
                            FlexParams::new(1.0, CrossAxisAlignment::Start),
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
