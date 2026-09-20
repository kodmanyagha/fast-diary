use druid::{
    tests::harness::Harness,
    widget::{Controller, SizedBox},
    Env, Event, EventCtx, Modifiers, MouseButton, MouseButtons, MouseEvent, Point, Vec2, Widget,
    WidgetExt,
};
use fast_diary::{
    consts::druid_selector, modal::app_state::AppState,
    view::widget::persistent_split::PersistentSplit,
};

const WINDOW_WIDTH: f64 = 400.0;
const MIN_FIRST_WIDTH: f64 = 100.0;
const MIN_SECOND_WIDTH: f64 = 100.0;

struct SaveRecorder;

impl<W: Widget<AppState>> Controller<AppState, W> for SaveRecorder {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
        env: &Env,
    ) {
        if let Event::Command(cmd) = event {
            if cmd.is(druid_selector::SETTINGS_SAVE) {
                app_state.status_message.push_str("save,");
            }
        }

        child.event(ctx, event, app_state, env)
    }
}

fn mouse_event(x: f64) -> MouseEvent {
    let pos = Point::new(x, 200.0);

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

fn split_test(initial_ratio: f64, scenario: impl FnMut(&mut Harness<AppState>)) {
    let mut app_state = AppState::new();
    app_state.list_split_ratio = initial_ratio;
    let split = PersistentSplit::columns(
        SizedBox::empty().expand(),
        SizedBox::empty().expand(),
        AppState::list_split_ratio,
    )
    .min_widths(MIN_FIRST_WIDTH, MIN_SECOND_WIDTH)
    .controller(SaveRecorder);

    Harness::create_simple(app_state, split, scenario);
}

fn drag_bar(harness: &mut Harness<AppState>, from_x: f64, to_x: f64) {
    harness.event(Event::MouseMove(mouse_event(from_x)));
    harness.event(Event::MouseDown(mouse_event(from_x)));
    harness.event(Event::MouseMove(mouse_event((from_x + to_x) / 2.0)));
    harness.event(Event::MouseMove(mouse_event(to_x)));
    harness.event(Event::MouseUp(mouse_event(to_x)));
}

fn open_window(harness: &mut Harness<AppState>) {
    harness.send_initial_events();
    harness.just_layout();
}

#[test]
fn dragging_the_bar_updates_the_ratio_and_asks_for_the_settings_to_be_saved() {
    split_test(0.5, |harness| {
        open_window(harness);

        drag_bar(harness, 197.0, 137.0);

        let ratio = harness.data().list_split_ratio;
        assert!((ratio - 0.35).abs() < 0.01, "ratio was {ratio}");
        assert_eq!(harness.data().status_message, "save,");
    });
}

#[test]
fn the_bar_cannot_be_dragged_below_the_minimum_widths() {
    split_test(0.5, |harness| {
        open_window(harness);

        drag_bar(harness, 197.0, 5.0);
        let narrowest = harness.data().list_split_ratio;
        drag_bar(harness, 103.0, 395.0);
        let widest = harness.data().list_split_ratio;

        let available_width = WINDOW_WIDTH - 6.0;
        assert_eq!(narrowest, MIN_FIRST_WIDTH / available_width);
        assert_eq!(
            widest,
            (available_width - MIN_SECOND_WIDTH) / available_width
        );
    });
}

#[test]
fn pressing_next_to_the_bar_changes_nothing() {
    split_test(0.5, |harness| {
        open_window(harness);

        drag_bar(harness, 60.0, 300.0);

        assert_eq!(harness.data().list_split_ratio, 0.5);
        assert_eq!(harness.data().status_message, "");
    });
}

#[test]
fn a_ratio_that_was_loaded_is_where_the_bar_starts() {
    split_test(0.6, |harness| {
        open_window(harness);

        drag_bar(harness, 236.0, 236.0);

        let ratio = harness.data().list_split_ratio;
        assert!((ratio - 0.6).abs() < 0.005, "ratio was {ratio}");
    });
}
