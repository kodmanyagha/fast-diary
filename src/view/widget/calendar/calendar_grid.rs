use std::collections::HashMap;

use chrono::{Local, NaiveDate};
use druid::{
    keyboard_types::Key,
    kurbo::Circle,
    piet::{Text, TextLayout, TextLayoutBuilder},
    theme, BoxConstraints, Color, Cursor, Data, Env, Event, EventCtx, FontFamily, LayoutCtx,
    LifeCycle, LifeCycleCtx, PaintCtx, Point, Rect, RenderContext, Size, UpdateCtx, Vec2, Widget,
};

use crate::{
    consts::druid_selector,
    modal::{
        app_state::AppState,
        app_state_utils::{entries_by_day, relative_diary, DayEntries},
        state::{calendar_month::DAYS_IN_WEEK, diary_list_item::DiaryListItem},
    },
    utils::localization::{month_short_name, weekday_short_name},
};

use super::grid_geometry::GridGeometry;

const ACCENT_COLOR: Color = Color::rgb8(70, 130, 180);
const CELL_GAP: f64 = 1.5;
const CELL_CORNER_RADIUS: f64 = 4.0;
const WEEKDAY_FONT_SIZE: f64 = 11.0;
const MONTH_TITLE_FONT_SIZE: f64 = 13.0;
const DAY_FONT_SIZE: f64 = 12.0;
const MAX_ENTRY_DOTS: usize = 3;
const ENTRY_DOT_RADIUS: f64 = 1.4;
const ENTRY_DOT_SPACING: f64 = 5.0;
const ENTRY_DOT_BOTTOM_OFFSET: f64 = 3.0;
const DAY_TEXT_LIFT_WITH_ENTRIES: f64 = 2.0;
const TODAY_OUTLINE_WIDTH: f64 = 1.0;

/// The index of a month among the months that the calendar shows and the index of a cell in it.
type CellPosition = (usize, usize);

/// Months in a row with their days, which mark the days having diaries and open their diaries one
/// after the other when a day is clicked, or an empty diary for a day that has none yet.
pub struct CalendarGrid {
    entries: HashMap<NaiveDate, DayEntries>,
    hovered_cell: Option<CellPosition>,
}

impl CalendarGrid {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            hovered_cell: None,
        }
    }

    fn geometry(width: f64, app_state: &AppState) -> GridGeometry {
        GridGeometry::new(width, app_state.calendar_month.window_rows())
    }

    fn refresh_entries(&mut self, app_state: &AppState) {
        self.entries = app_state
            .calendar_month
            .window()
            .iter()
            .flat_map(|month| entries_by_day(&app_state.diaries, *month))
            .collect();
    }

    fn date_in_cell(
        app_state: &AppState,
        (month_index, cell_index): CellPosition,
    ) -> Option<NaiveDate> {
        app_state
            .calendar_month
            .shifted(month_index as i32)
            .date_at_cell(cell_index)
    }

    fn open_diary(ctx: &mut EventCtx, diary: DiaryListItem) {
        ctx.submit_command(druid_selector::DIARY_SAVE_CURRENT);
        ctx.submit_command(druid_selector::DIARY_SET_CURRENT.with(diary));
    }

    fn start_draft(ctx: &mut EventCtx, date: NaiveDate) {
        ctx.submit_command(druid_selector::DIARY_SAVE_CURRENT);
        ctx.submit_command(druid_selector::DIARY_START_DRAFT.with(date));
    }

    fn open_relative_diary(ctx: &mut EventCtx, app_state: &AppState, offset: isize) {
        if let Some(diary) = relative_diary(app_state, offset) {
            Self::open_diary(ctx, diary);
        }
    }

    fn cell_fill(is_selected: bool, has_entries: bool, is_hovered: bool) -> Option<Color> {
        match (is_selected, has_entries, is_hovered) {
            (true, _, _) => Some(ACCENT_COLOR),
            (false, true, true) => Some(ACCENT_COLOR.with_alpha(0.55)),
            (false, true, false) => Some(ACCENT_COLOR.with_alpha(0.3)),
            (false, false, true) => Some(ACCENT_COLOR.with_alpha(0.15)),
            (false, false, false) => None,
        }
    }

    fn draw_centered_text(
        ctx: &mut PaintCtx,
        text: &str,
        area: Rect,
        font_size: f64,
        color: Color,
    ) {
        let Ok(layout) = ctx
            .text()
            .new_text_layout(text.to_string())
            .font(FontFamily::SYSTEM_UI, font_size)
            .text_color(color)
            .build()
        else {
            return;
        };

        let text_size = layout.size();
        ctx.draw_text(
            &layout,
            Point::new(
                area.center().x - text_size.width / 2.0,
                area.center().y - text_size.height / 2.0,
            ),
        );
    }

    fn draw_entry_dots(ctx: &mut PaintCtx, cell: Rect, entry_count: usize, color: &Color) {
        let dot_count = entry_count.min(MAX_ENTRY_DOTS);
        let first_dot_offset = (dot_count as f64 - 1.0) / 2.0;

        (0..dot_count).for_each(|dot| {
            let center = Point::new(
                cell.center().x + (dot as f64 - first_dot_offset) * ENTRY_DOT_SPACING,
                cell.y1 - ENTRY_DOT_BOTTOM_OFFSET,
            );
            ctx.fill(Circle::new(center, ENTRY_DOT_RADIUS), color);
        });
    }
}

impl Default for CalendarGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget<AppState> for CalendarGrid {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, app_state: &mut AppState, _env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(druid_selector::DIARY_BROWSER_FOCUS) => {
                ctx.request_focus();
            }
            Event::MouseMove(mouse_event) => {
                let hovered_cell = Self::geometry(ctx.size().width, app_state)
                    .cell_at(mouse_event.pos)
                    .filter(|cell| Self::date_in_cell(app_state, *cell).is_some());

                if hovered_cell != self.hovered_cell {
                    self.hovered_cell = hovered_cell;
                    ctx.request_paint();
                }
                ctx.set_cursor(&if hovered_cell.is_some() {
                    Cursor::Pointer
                } else {
                    Cursor::Arrow
                });
            }
            Event::MouseDown(mouse_event) => {
                ctx.request_focus();
                let clicked_date = Self::geometry(ctx.size().width, app_state)
                    .cell_at(mouse_event.pos)
                    .and_then(|cell| Self::date_in_cell(app_state, cell));

                if let Some(date) = clicked_date {
                    let opened_file_name = app_state
                        .current_diary
                        .is_selected
                        .then_some(app_state.current_diary.diary.file_name.as_str());

                    match self.entries.get(&date) {
                        Some(day_entries) => Self::open_diary(
                            ctx,
                            day_entries.diary_to_open(opened_file_name).clone(),
                        ),
                        None => Self::start_draft(ctx, date),
                    }
                }
            }
            Event::KeyDown(key_event) => {
                let offset = match key_event.key {
                    Key::ArrowLeft | Key::ArrowUp => Some(1),
                    Key::ArrowRight | Key::ArrowDown => Some(-1),
                    _ => None,
                };

                if let Some(offset) = offset {
                    Self::open_relative_diary(ctx, app_state, offset);
                    ctx.set_handled();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        app_state: &AppState,
        _env: &Env,
    ) {
        match event {
            LifeCycle::WidgetAdded => {
                self.refresh_entries(app_state);
                ctx.submit_command(druid_selector::DIARY_BROWSER_FOCUS);
            }
            LifeCycle::BuildFocusChain => ctx.register_for_focus(),
            LifeCycle::HotChanged(false) => {
                if self.hovered_cell.take().is_some() {
                    ctx.request_paint();
                }
            }
            LifeCycle::FocusChanged(_) => ctx.request_paint(),
            _ => {}
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_app_state: &AppState,
        app_state: &AppState,
        _env: &Env,
    ) {
        let entries_changed = !old_app_state.diaries.same(&app_state.diaries)
            || old_app_state.calendar_month != app_state.calendar_month;

        if entries_changed {
            self.refresh_entries(app_state);
            ctx.request_layout();
        }
        if entries_changed || !old_app_state.current_diary.same(&app_state.current_diary) {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        app_state: &AppState,
        _env: &Env,
    ) -> Size {
        bc.constrain(Self::geometry(bc.max().width, app_state).size())
    }

    fn paint(&mut self, ctx: &mut PaintCtx, app_state: &AppState, env: &Env) {
        let geometry = Self::geometry(ctx.size().width, app_state);
        let text_color = env.get(theme::TEXT_COLOR);
        let dimmed_color = env.get(theme::DISABLED_TEXT_COLOR);
        let today = Local::now().date_naive();
        let opened_date = app_state
            .current_diary
            .is_selected
            .then(|| app_state.current_diary.diary.date.local_date());

        (0..DAYS_IN_WEEK).for_each(|column| {
            Self::draw_centered_text(
                ctx,
                &weekday_short_name(app_state.language, column),
                geometry.weekday_label_rect(column),
                WEEKDAY_FONT_SIZE,
                dimmed_color,
            )
        });

        app_state
            .calendar_month
            .window()
            .iter()
            .enumerate()
            .for_each(|(month_index, month)| {
                Self::draw_centered_text(
                    ctx,
                    &month.title(&month_short_name(app_state.language, month.month_number())),
                    geometry.month_title_rect(month_index),
                    MONTH_TITLE_FONT_SIZE,
                    text_color,
                );

                (0..month.week_rows() * DAYS_IN_WEEK).for_each(|cell_index| {
                    let Some(date) = month.date_at_cell(cell_index) else {
                        return;
                    };

                    let day_entries = self.entries.get(&date);
                    let is_opened = opened_date == Some(date);
                    let cell = geometry.cell_rect(month_index, cell_index).inset(-CELL_GAP);
                    let rounded_cell = cell.to_rounded_rect(CELL_CORNER_RADIUS);

                    if let Some(fill) = Self::cell_fill(
                        is_opened,
                        day_entries.is_some(),
                        self.hovered_cell == Some((month_index, cell_index)),
                    ) {
                        ctx.fill(rounded_cell, &fill);
                    }
                    if date == today {
                        ctx.stroke(rounded_cell, &text_color, TODAY_OUTLINE_WIDTH);
                    }

                    let day_color = if day_entries.is_some() {
                        text_color
                    } else {
                        dimmed_color
                    };
                    let day_text_area = if day_entries.is_some() {
                        cell - Vec2::new(0.0, DAY_TEXT_LIFT_WITH_ENTRIES)
                    } else {
                        cell
                    };
                    Self::draw_centered_text(
                        ctx,
                        &date.format("%-d").to_string(),
                        day_text_area,
                        DAY_FONT_SIZE,
                        day_color,
                    );

                    if let Some(day_entries) = day_entries {
                        Self::draw_entry_dots(ctx, cell, day_entries.count(), &text_color);
                    }
                });
            });
    }
}
