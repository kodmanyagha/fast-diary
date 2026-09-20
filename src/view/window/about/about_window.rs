use druid::{
    commands, theme,
    widget::{Button, CrossAxisAlignment, Flex, Label, LineBreaking},
    Application, Insets, Size, Widget, WidgetExt, WindowDesc,
};

use crate::{
    consts::project_info::{
        AUTHOR_EMAIL, AUTHOR_NAME, ETHEREUM_ADDRESS, ISSUES_URL, REPOSITORY_URL, SOLANA_ADDRESS,
    },
    modal::app_state::AppState,
    utils::localization::{text, translate},
};

const ABOUT_WINDOW_SIZE: Size = Size::new(740.0, 420.0);
const APP_NAME_FONT_SIZE: f64 = 24.0;
const WINDOW_PADDING: f64 = 20.0;
const SECTION_SPACING: f64 = 18.0;
const ROW_SPACING: f64 = 8.0;
const ROW_LABEL_WIDTH: f64 = 132.0;

fn copy_to_clipboard(value: &str) {
    Application::global().clipboard().put_string(value);
}

/// Builds a row that names `value` with the translation under `label_key` and has a button that
/// copies the value, because the text of a label cannot be selected.
fn build_copyable_row(label_key: &'static str, value: &'static str) -> impl Widget<AppState> {
    Flex::row()
        .with_child(Label::new(text(label_key)).fix_width(ROW_LABEL_WIDTH))
        .with_flex_child(
            Label::<AppState>::new(value).with_line_break_mode(LineBreaking::WordWrap),
            1.0,
        )
        .with_spacer(ROW_SPACING)
        .with_child(
            Button::new(text("about-copy"))
                .on_click(move |_ctx, _app_state: &mut AppState, _env| copy_to_clipboard(value)),
        )
}

fn build_author_row() -> impl Widget<AppState> {
    Flex::row()
        .with_child(Label::new(text("about-author")).fix_width(ROW_LABEL_WIDTH))
        .with_child(Label::<AppState>::new(AUTHOR_NAME))
}

pub fn build_ui() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Label::new(text("app-name"))
                .with_font(theme::UI_FONT_BOLD)
                .with_text_size(APP_NAME_FONT_SIZE),
        )
        .with_spacer(ROW_SPACING)
        .with_child(Label::dynamic(|app_state: &AppState, _env| {
            format!(
                "{} {}",
                translate(app_state.language, "about-version"),
                env!("CARGO_PKG_VERSION")
            )
        }))
        .with_spacer(ROW_SPACING)
        .with_child(Label::new(text("about-description")))
        .with_spacer(SECTION_SPACING)
        .with_child(build_author_row())
        .with_spacer(ROW_SPACING)
        .with_child(build_copyable_row("about-email", AUTHOR_EMAIL))
        .with_spacer(ROW_SPACING)
        .with_child(build_copyable_row("about-repository", REPOSITORY_URL))
        .with_spacer(ROW_SPACING)
        .with_child(build_copyable_row("about-issues", ISSUES_URL))
        .with_spacer(SECTION_SPACING)
        .with_child(Label::new(text("about-support")).with_font(theme::UI_FONT_BOLD))
        .with_spacer(ROW_SPACING)
        .with_child(build_copyable_row("about-ethereum", ETHEREUM_ADDRESS))
        .with_spacer(ROW_SPACING)
        .with_child(build_copyable_row("about-solana", SOLANA_ADDRESS))
        .with_flex_spacer(1.0)
        .with_child(Button::new(text("about-close")).on_click(
            |ctx, _app_state: &mut AppState, _env| ctx.submit_command(commands::CLOSE_WINDOW),
        ))
        .padding(Insets::uniform(WINDOW_PADDING))
        .expand()
}

/// Describes the window that shows the name, the version and the purpose of the application, who
/// made it, where to report problems and how to support it.
pub fn build_about_window() -> WindowDesc<AppState> {
    WindowDesc::new(build_ui())
        .title(text("menu-help-about"))
        .window_size(ABOUT_WINDOW_SIZE)
        .resizable(false)
}
