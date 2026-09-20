use chrono::Local;
use druid::{
    tests::harness::Harness,
    widget::{Align, Controller},
    Env, Event, EventCtx, Modifiers, MouseButton, MouseButtons, MouseEvent, Point, UnitPoint, Vec2,
    Widget, WidgetExt,
};
use fast_diary::{
    consts::druid_selector,
    modal::{app_state::AppState, state::calendar_month::CalendarMonth},
    view::widget::calendar::{build_calendar, build_today_button},
};

struct StartedDraftRecorder;

impl<W: Widget<AppState>> Controller<AppState, W> for StartedDraftRecorder {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
        env: &Env,
    ) {
        if let Event::Command(cmd) = event {
            if let Some(date) = cmd.get(druid_selector::DIARY_START_DRAFT) {
                app_state.status_message = date.to_string();
            }
        }

        child.event(ctx, event, app_state, env)
    }
}

fn mouse_event(pos: Point) -> MouseEvent {
    MouseEvent {
        pos,
        window_pos: pos,
        buttons: MouseButtons::new().with(MouseButton::Left),
        mods: Modifiers::empty(),
        count: 1,
        focus: false,
        button: MouseButton::Left,
        wheel_delta: Vec2::ZERO,
    }
}

fn click(harness: &mut Harness<AppState>, pos: Point) {
    harness.event(Event::MouseMove(mouse_event(pos)));
    harness.event(Event::MouseDown(mouse_event(pos)));
    harness.event(Event::MouseUp(mouse_event(pos)));
}

fn open_window(harness: &mut Harness<AppState>) {
    harness.send_initial_events();
    harness.just_layout();
}

#[test]
fn the_calendar_starts_with_the_current_month_in_the_middle() {
    assert_eq!(
        AppState::new().calendar_month,
        CalendarMonth::current().shifted(-1)
    );
}

#[test]
fn the_arrows_move_the_calendar_by_the_three_months_that_it_shows() {
    let mut app_state = AppState::new();
    let start = CalendarMonth::current().shifted(-1);
    app_state.calendar_month = start;
    let previous_arrow = Point::new(15.0, 15.0);
    let next_arrow = Point::new(385.0, 15.0);

    Harness::create_simple(app_state, build_calendar(), |harness| {
        open_window(harness);

        click(harness, next_arrow);
        assert_eq!(harness.data().calendar_month, start.shifted(3));

        click(harness, next_arrow);
        assert_eq!(harness.data().calendar_month, start.shifted(6));

        click(harness, previous_arrow);
        click(harness, previous_arrow);
        click(harness, previous_arrow);
        assert_eq!(harness.data().calendar_month, start.shifted(-3));
    });
}

#[test]
fn the_today_button_selects_todays_date() {
    let root =
        Align::new(UnitPoint::TOP_LEFT, build_today_button()).controller(StartedDraftRecorder);

    Harness::create_simple(AppState::new(), root, |harness| {
        open_window(harness);

        click(harness, Point::new(50.0, 12.0));

        assert_eq!(
            harness.data().status_message,
            Local::now().date_naive().to_string()
        );
    });
}
