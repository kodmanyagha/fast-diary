pub mod calendar_grid;
pub mod grid_geometry;

use chrono::Local;
use druid::{
    widget::{Button, Flex, Label, Scroll},
    Env, EventCtx, Widget, WidgetExt,
};

use crate::{
    consts::druid_selector,
    modal::{app_state::AppState, state::calendar_month::MONTHS_SHOWN},
    utils::localization::{month_short_name, text},
};

use calendar_grid::CalendarGrid;

const SECTION_SPACING: f64 = 6.0;
const WINDOW_TITLE_TEXT_SIZE: f64 = 13.0;
const SCROLL_BAR_ALLOWANCE: f64 = 10.0;

fn shift_calendar(app_state: &mut AppState, direction: i32) {
    app_state.calendar_month = app_state
        .calendar_month
        .shifted(direction * MONTHS_SHOWN as i32);
}

fn build_window_title() -> impl Widget<AppState> {
    Label::dynamic(|app_state: &AppState, _env: &Env| {
        let first_month = app_state.calendar_month;
        let last_month = first_month.shifted(MONTHS_SHOWN as i32 - 1);

        first_month.window_title(
            &month_short_name(app_state.language, first_month.month_number()),
            &month_short_name(app_state.language, last_month.month_number()),
        )
    })
    .with_text_size(WINDOW_TITLE_TEXT_SIZE)
    .center()
}

/// Builds the button that selects today: it opens the latest diary of today, or an empty diary
/// when nothing was written today yet, and lets the calendar show today.
pub fn build_today_button() -> impl Widget<AppState> {
    Button::new(text("calendar-today"))
        .on_click(
            |ctx: &mut EventCtx, _app_state: &mut AppState, _env: &Env| {
                ctx.submit_command(druid_selector::DIARY_SAVE_CURRENT);
                ctx.submit_command(
                    druid_selector::DIARY_START_DRAFT.with(Local::now().date_naive()),
                );
            },
        )
        .expand_width()
}

/// Builds the calendar: a bar to move by the number of months that it shows, the months, and the
/// button that selects today.
pub fn build_calendar() -> impl Widget<AppState> {
    let previous_months = Button::new("<").on_click(|_ctx, app_state: &mut AppState, _env| {
        shift_calendar(app_state, -1);
    });
    let next_months = Button::new(">").on_click(|_ctx, app_state: &mut AppState, _env| {
        shift_calendar(app_state, 1);
    });

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(previous_months)
                .with_flex_child(build_window_title(), 1.0)
                .with_child(next_months)
                .expand_width(),
        )
        .with_spacer(SECTION_SPACING)
        .with_flex_child(
            Scroll::new(CalendarGrid::new().expand_width().padding((
                0.0,
                0.0,
                SCROLL_BAR_ALLOWANCE,
                0.0,
            )))
            .vertical()
            .expand_width(),
            1.0,
        )
        .with_spacer(SECTION_SPACING)
        .with_child(build_today_button())
        .expand_height()
}
