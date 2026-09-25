use std::{cell::Cell, rc::Rc};

use druid::{
    tests::harness::Harness,
    widget::{Controller, Scroll},
    Env, Event, EventCtx, Modifiers, MouseButton, MouseButtons, MouseEvent, Point, Selector, Size,
    Vec2, Widget, WidgetExt,
};
use fast_diary::view::widget::diary_editor::DiaryEditor;

const EDITOR_SIZE: Size = Size::new(300.0, 130.0);
const FLUSH: Selector<()> = Selector::new("test.flush");

/// Records how far the editor's text has been scrolled up inside the enclosing `Scroll`.
struct ScrollOffsetRecorder(Rc<Cell<f64>>);

impl Controller<String, DiaryEditor> for ScrollOffsetRecorder {
    fn event(
        &mut self,
        child: &mut DiaryEditor,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
        self.0.set(-ctx.window_origin().y);
        child.event(ctx, event, data, env)
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

fn click(harness: &mut Harness<String>, pos: Point) {
    harness.event(Event::MouseMove(mouse_event(pos)));
    harness.event(Event::MouseDown(mouse_event(pos)));
    harness.event(Event::MouseUp(mouse_event(pos)));
}

/// Lets the editor handle the requests it made during layout, then returns the scroll offset.
fn settled_scroll_offset(harness: &mut Harness<String>, offset: &Rc<Cell<f64>>) -> f64 {
    harness.just_layout();
    harness.submit_command(FLUSH);
    harness.just_layout();
    harness.submit_command(FLUSH);
    offset.get()
}

fn editor_test(scenario: impl FnMut(&mut Harness<String>, &Rc<Cell<f64>>)) {
    let text: String = (1..=30).map(|line| format!("line {line}\n")).collect();
    let offset = Rc::new(Cell::new(0.0));
    let root = Scroll::new(DiaryEditor::new().controller(ScrollOffsetRecorder(offset.clone())))
        .vertical()
        .content_must_fill(true);
    let mut scenario = scenario;

    Harness::create_with_render(
        text,
        root,
        EDITOR_SIZE,
        |harness| {
            harness.send_initial_events();
            harness.just_layout();
            scenario(harness, &offset);
        },
        |_| {},
    );
}

#[test]
fn a_line_cut_by_the_bottom_edge_scrolls_fully_into_view_with_room_below() {
    editor_test(|harness, offset| {
        assert_eq!(settled_scroll_offset(harness, offset), 0.0);

        click(harness, Point::new(20.0, EDITOR_SIZE.height - 3.0));

        assert!(settled_scroll_offset(harness, offset) > 25.0);
    });
}

#[test]
fn moving_the_caret_within_the_visible_lines_does_not_scroll() {
    editor_test(|harness, offset| {
        click(harness, Point::new(20.0, 40.0));

        assert_eq!(settled_scroll_offset(harness, offset), 0.0);
    });
}
