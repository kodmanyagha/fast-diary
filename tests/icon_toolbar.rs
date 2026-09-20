use druid::{
    tests::harness::Harness,
    widget::{Align, Controller},
    Env, Event, EventCtx, Modifiers, MouseButton, MouseButtons, MouseEvent, Point, UnitPoint, Vec2,
    Widget, WidgetExt,
};
use fast_diary::{
    consts::druid_selector,
    modal::{app_state::AppState, state::app_pages::AppPages},
    view::page::diary::widgets::icon_toolbar::build_icon_toolbar,
};

const BUTTON_SIZE: f64 = 28.0;
const BUTTON_SPACING: f64 = 4.0;

struct CommandRecorder;

impl<W: Widget<AppState>> Controller<AppState, W> for CommandRecorder {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
        env: &Env,
    ) {
        if let Event::Command(cmd) = event {
            if cmd.is(druid_selector::CREATE_NEW_DIARY) {
                app_state.status_message.push_str("create,");
            } else if cmd.is(druid_selector::FOLDER_LOCK) {
                app_state.status_message.push_str("lock,");
            }
        }

        child.event(ctx, event, app_state, env)
    }
}

fn center_of_button(index: usize) -> Point {
    Point::new(
        index as f64 * (BUTTON_SIZE + BUTTON_SPACING) + BUTTON_SIZE / 2.0,
        BUTTON_SIZE / 2.0,
    )
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

fn click_button(index: usize, scenario: impl FnOnce(&AppState)) {
    let mut app_state = AppState::new();
    app_state.page = AppPages::Diary;
    let position = center_of_button(index);
    let mut scenario = Some(scenario);

    Harness::create_simple(
        app_state,
        Align::new(UnitPoint::TOP_LEFT, build_icon_toolbar()).controller(CommandRecorder),
        move |harness| {
            harness.send_initial_events();
            harness.just_layout();
            harness.event(Event::MouseMove(mouse_event(position)));
            harness.event(Event::MouseDown(mouse_event(position)));
            harness.event(Event::MouseUp(mouse_event(position)));

            if let Some(scenario) = scenario.take() {
                scenario(harness.data());
            }
        },
    );
}

#[test]
fn the_plus_button_creates_a_diary() {
    click_button(0, |app_state| {
        assert_eq!(app_state.status_message, "create,");
    });
}

#[test]
fn the_cog_button_opens_the_settings() {
    click_button(1, |app_state| {
        assert_eq!(app_state.page, AppPages::Settings);
        assert_eq!(app_state.status_message, "");
    });
}

#[test]
fn the_logout_button_closes_the_folder() {
    click_button(2, |app_state| {
        assert_eq!(app_state.status_message, "lock,");
    });
}
