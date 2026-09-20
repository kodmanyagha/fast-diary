use std::{cell::Cell, cell::RefCell, rc::Rc};

use druid::{
    tests::harness::Harness, widget::Controller, BoxConstraints, Env, Event, EventCtx,
    InternalEvent, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Selector, Size, TimerToken,
    UpdateCtx, Widget, WidgetExt, WidgetId,
};
use fast_diary::{modal::app_state::AppState, view::widget::markdown_preview::MarkdownPreview};

const CHANGE_TEXT: Selector<String> = Selector::new("test.change_text");
const SWITCH_DIARY: Selector<(String, String)> = Selector::new("test.switch_diary");

struct DataWriter;

impl<W: Widget<AppState>> Controller<AppState, W> for DataWriter {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
        env: &Env,
    ) {
        if let Event::Command(cmd) = event {
            if let Some(text) = cmd.get(CHANGE_TEXT) {
                app_state.txt_diary = text.clone();
            }
            if let Some((file_name, text)) = cmd.get(SWITCH_DIARY) {
                app_state.current_diary.is_selected = true;
                app_state.current_diary.diary.file_name = file_name.clone();
                app_state.txt_diary = text.clone();
            }
        }

        child.event(ctx, event, app_state, env)
    }
}

/// Passes everything on to a preview and notes what the preview shows and waits for.
struct PreviewProbe {
    preview: MarkdownPreview,
    shown_text: Rc<RefCell<String>>,
    update_timer: Rc<Cell<TimerToken>>,
}

impl PreviewProbe {
    fn note(&self) {
        *self.shown_text.borrow_mut() = self.preview.rendered_text().to_string();
        self.update_timer.set(self.preview.update_timer());
    }
}

impl Widget<AppState> for PreviewProbe {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        self.preview.event(ctx, event, data, env);
        self.note();
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppState, env: &Env) {
        self.preview.lifecycle(ctx, event, data, env);
        self.note();
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppState, data: &AppState, env: &Env) {
        self.preview.update(ctx, old_data, data, env);
        self.note();
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppState,
        env: &Env,
    ) -> Size {
        self.preview.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppState, env: &Env) {
        self.preview.paint(ctx, data, env)
    }
}

fn preview_test(scenario: impl FnMut(&mut Harness<AppState>, &PreviewHandles)) {
    let shown_text = Rc::new(RefCell::new(String::new()));
    let update_timer = Rc::new(Cell::new(TimerToken::INVALID));
    let preview_id = WidgetId::next();
    let handles = PreviewHandles {
        shown_text: shown_text.clone(),
        update_timer: update_timer.clone(),
        preview_id,
    };
    let root = PreviewProbe {
        preview: MarkdownPreview::new(),
        shown_text,
        update_timer,
    }
    .with_id(preview_id)
    .controller(DataWriter);

    let mut app_state = AppState::new();
    app_state.txt_diary = "first".to_string();
    let mut scenario = scenario;
    Harness::create_simple(app_state, root, move |harness| {
        harness.send_initial_events();
        harness.just_layout();
        scenario(harness, &handles);
    });
}

struct PreviewHandles {
    shown_text: Rc<RefCell<String>>,
    update_timer: Rc<Cell<TimerToken>>,
    preview_id: WidgetId,
}

impl PreviewHandles {
    fn shown(&self) -> String {
        self.shown_text.borrow().clone()
    }

    fn fire(&self, harness: &mut Harness<AppState>, token: TimerToken) {
        harness.event(Event::Internal(InternalEvent::RouteTimer(
            token,
            self.preview_id,
        )));
    }
}

#[test]
fn the_first_text_is_shown_at_once() {
    preview_test(|_harness, preview| {
        assert_eq!(preview.shown(), "first");
    });
}

#[test]
fn an_edit_is_shown_only_after_the_delay() {
    preview_test(|harness, preview| {
        harness.submit_command(CHANGE_TEXT.with("second".to_string()));

        assert_eq!(preview.shown(), "first");
        let token = preview.update_timer.get();
        assert_ne!(token, TimerToken::INVALID);

        preview.fire(harness, token);
        assert_eq!(preview.shown(), "second");
        assert_eq!(preview.update_timer.get(), TimerToken::INVALID);
    });
}

#[test]
fn only_the_timer_of_the_last_edit_updates_the_preview() {
    preview_test(|harness, preview| {
        harness.submit_command(CHANGE_TEXT.with("second".to_string()));
        let early_token = preview.update_timer.get();
        harness.submit_command(CHANGE_TEXT.with("third".to_string()));
        let last_token = preview.update_timer.get();
        assert_ne!(early_token, last_token);

        preview.fire(harness, early_token);
        assert_eq!(preview.shown(), "first");

        preview.fire(harness, last_token);
        assert_eq!(preview.shown(), "third");
    });
}

#[test]
fn another_diary_is_shown_at_once_and_drops_the_waiting_edit() {
    preview_test(|harness, preview| {
        harness.submit_command(CHANGE_TEXT.with("edit".to_string()));
        let waiting_token = preview.update_timer.get();

        harness
            .submit_command(SWITCH_DIARY.with(("other.md".to_string(), "other text".to_string())));

        assert_eq!(preview.shown(), "other text");
        assert_eq!(preview.update_timer.get(), TimerToken::INVALID);

        preview.fire(harness, waiting_token);
        assert_eq!(preview.shown(), "other text");
    });
}
