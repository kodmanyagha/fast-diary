mod undo_history;

use std::time::Instant;

use druid::{
    commands,
    text::Selection,
    theme,
    widget::{Scroll, TextBox},
    BoxConstraints, Code, Command, Data, Env, Event, EventCtx, Insets, KbKey, KeyEvent, LayoutCtx,
    LifeCycle, LifeCycleCtx, Modifiers, PaintCtx, Rect, Selector, Size, Target, UpdateCtx, Vec2,
    Widget, WidgetExt,
};

use crate::consts::druid_selector;

use self::undo_history::{EditKind, EditorSnapshot, UndoHistory};

const FOLLOW_CARET: Selector<()> = Selector::new("diary_editor.follow_caret");

/// Room kept visible above the caret's line when the editor scrolls to it.
const CARET_MARGIN_ABOVE: f64 = 8.0;
/// Room kept visible below the caret's line, so the line being typed never touches the bottom.
const CARET_MARGIN_BELOW: f64 = 28.0;

/// Returns the 0-based line of `text` that the byte `offset` falls on.
fn line_at(text: &str, offset: usize) -> usize {
    text.get(..offset).unwrap_or(text).matches('\n').count()
}

/// Returns `text` with an empty line opened below the line that the byte `caret` falls on,
/// together with the offset of that new line, wherever on its line the caret was.
fn with_line_opened_below(text: &str, caret: usize) -> (String, usize) {
    let line_end = text
        .get(caret..)
        .and_then(|rest| rest.find('\n'))
        .map_or(text.len(), |distance| caret + distance);
    let opened = [&text[..line_end], "\n", &text[line_end..]].concat();
    (opened, line_end + 1)
}

/// Tells whether `key` is Shift+Enter with no other modifier held.
fn is_open_line_below_shortcut(key: &KeyEvent) -> bool {
    key.key == KbKey::Enter
        && key.mods.shift()
        && !key.mods.ctrl()
        && !key.mods.alt()
        && !key.mods.meta()
}

/// A multi-line diary editor that is laid out at the full height of its text inside an outer
/// [`Scroll`], and scrolls that [`Scroll`] itself to keep the caret's line fully in view.
///
/// druid's own [`TextBox`] scrolling measures the caret without the text insets and without any
/// vertical margin, so the line being typed ends up partly hidden under the bottom edge.
///
/// druid's [`TextBox`] also keeps no edit history, so the editor records its own and answers the
/// undo and redo commands the text box sends for Ctrl+Z and Ctrl+Shift+Z.
pub struct DiaryEditor {
    text_box: TextBox<String>,
    is_caret_follow_due: bool,
    last_reported_cursor_line: Option<usize>,
    undo_history: UndoHistory,
    last_known_state: Option<EditorSnapshot>,
}

impl DiaryEditor {
    pub fn new() -> Self {
        Self {
            text_box: TextBox::multiline().with_line_wrapping(true),
            is_caret_follow_due: false,
            last_reported_cursor_line: None,
            undo_history: UndoHistory::default(),
            last_known_state: None,
        }
    }

    /// Returns the current selection, or a caret at the end of `text` while the input method
    /// holds the text.
    fn selection_or_end(&self, text: &str) -> Selection {
        let text_component = self.text_box.text();
        if text_component.can_read() {
            text_component.borrow().selection()
        } else {
            Selection::caret(text.len())
        }
    }

    /// Captures the text and the selection as they are now.
    fn snapshot_of(&self, text: &str) -> EditorSnapshot {
        EditorSnapshot::new(text, self.selection_or_end(text))
    }

    /// Records the change from the last known state to `data` as an undo step, if the text
    /// changed, and remembers `data` as the new known state.
    fn record_edit(&mut self, data: &str) {
        let current = self.snapshot_of(data);
        if let Some(before) = self.last_known_state.take() {
            if before.text != current.text {
                let kind = EditKind::between(&before.text, &current.text);
                self.undo_history.record(before, kind, Instant::now());
            }
        }
        self.last_known_state = Some(current);
    }

    /// Forgets the history when the text was replaced from outside the editor, such as when
    /// another diary is opened.
    fn forget_history_if_replaced(&mut self, data: &str) {
        let is_replaced = self
            .last_known_state
            .as_ref()
            .is_none_or(|known| known.text != data);
        if is_replaced {
            self.undo_history.clear();
            self.last_known_state = Some(self.snapshot_of(data));
        }
    }

    /// Shows `snapshot` in the editor, placing the selection where it was at that time.
    fn restore(&mut self, ctx: &mut EventCtx, data: &mut String, snapshot: EditorSnapshot) {
        data.clone_from(&snapshot.text);
        let text_component = self.text_box.text_mut();
        if text_component.can_write() {
            if let Some(invalidation) = text_component
                .borrow_mut()
                .set_selection(snapshot.selection)
            {
                ctx.invalidate_text_input(invalidation);
            }
        }
        self.last_known_state = Some(snapshot);
        self.schedule_caret_follow();
        ctx.request_layout();
    }

    /// Undoes or redoes one step, depending on `is_redo`. Does nothing while the input method
    /// is composing text or there is no step to go to.
    fn step_through_history(&mut self, ctx: &mut EventCtx, data: &mut String, is_redo: bool) {
        if self.text_box.text().is_composing() {
            return;
        }
        let current = self.snapshot_of(data);
        let target = if is_redo {
            self.undo_history.redo(current)
        } else {
            self.undo_history.undo(current)
        };
        if let Some(snapshot) = target {
            self.restore(ctx, data, snapshot);
        }
    }

    /// Opens an empty line below the caret's line and moves the caret onto it, recording the
    /// change as one undo step. Does nothing while the input method is composing text.
    fn open_line_below(&mut self, ctx: &mut EventCtx, data: &mut String) {
        if self.text_box.text().is_composing() {
            return;
        }
        let Some(caret) = self.caret() else {
            return;
        };
        let (opened, new_line_start) = with_line_opened_below(data, caret);
        *data = opened;
        let text_component = self.text_box.text_mut();
        if text_component.can_write() {
            if let Some(invalidation) = text_component
                .borrow_mut()
                .set_selection(Selection::caret(new_line_start))
            {
                ctx.invalidate_text_input(invalidation);
            }
        }
        self.record_edit(data);
        self.schedule_caret_follow();
        ctx.request_layout();
    }

    /// Returns the text offset of the caret, or `None` while the input method holds the text.
    fn caret(&self) -> Option<usize> {
        let text = self.text_box.text();
        text.can_read().then(|| text.borrow().selection().active)
    }

    /// Returns the area around the caret's line, in this widget's coordinates, that has to be
    /// visible.
    fn caret_area(&self, env: &Env) -> Option<Rect> {
        let text = self.text_box.text();
        text.can_read().then(|| {
            let session = text.borrow();
            let caret_line = session.cursor_line_for_text_position(session.selection().active);
            let insets = env.get(theme::TEXTBOX_INSETS);

            Rect::from_points(caret_line.p0, caret_line.p1)
                + Vec2::new(insets.x0, insets.y0)
                + Insets::new(0.0, CARET_MARGIN_ABOVE, 1.0, CARET_MARGIN_BELOW)
        })
    }

    /// Makes the editor scroll to the caret once the next layout has placed the text.
    fn schedule_caret_follow(&mut self) {
        self.is_caret_follow_due = true;
    }

    /// Tells the rest of the app which line the caret is on whenever that line changes.
    fn report_cursor_line(&mut self, ctx: &mut EventCtx, data: &str) {
        let Some(caret) = self.caret() else {
            return;
        };
        let cursor_line = line_at(data, caret);
        if self.last_reported_cursor_line != Some(cursor_line) {
            self.last_reported_cursor_line = Some(cursor_line);
            ctx.submit_command(Command::new(
                druid_selector::EDITOR_CURSOR_LINE_CHANGED,
                cursor_line,
                Target::Global,
            ));
        }
    }
}

impl Default for DiaryEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget<String> for DiaryEditor {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut String, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(FOLLOW_CARET) => {
                if let Some(area) = self.caret_area(env) {
                    ctx.scroll_area_to_view(area);
                }
                ctx.set_handled();
                return;
            }
            Event::Command(cmd)
                if (cmd.is(commands::UNDO) || cmd.is(commands::REDO)) && ctx.has_focus() =>
            {
                self.forget_history_if_replaced(data);
                self.step_through_history(ctx, data, cmd.is(commands::REDO));
                ctx.set_handled();
                self.report_cursor_line(ctx, data);
                return;
            }
            Event::KeyDown(key)
                if key.code == Code::KeyY && key.mods.contains(Modifiers::CONTROL) =>
            {
                self.forget_history_if_replaced(data);
                self.step_through_history(ctx, data, true);
                ctx.set_handled();
                self.report_cursor_line(ctx, data);
                return;
            }
            Event::KeyDown(key) if is_open_line_below_shortcut(key) && ctx.has_focus() => {
                self.forget_history_if_replaced(data);
                self.open_line_below(ctx, data);
                ctx.set_handled();
                self.report_cursor_line(ctx, data);
                return;
            }
            Event::KeyUp(key)
                if key.code == Code::KeyS && key.mods.contains(Modifiers::CONTROL) =>
            {
                ctx.submit_command(Command::new(
                    druid_selector::DIARY_SAVE_CURRENT,
                    (),
                    Target::Global,
                ));
            }
            _ => {}
        }

        self.forget_history_if_replaced(data);
        let caret_before = self.caret();
        self.text_box.event(ctx, event, data, env);
        self.record_edit(data);
        if self.caret() != caret_before {
            self.schedule_caret_follow();
            ctx.request_layout();
        }

        self.report_cursor_line(ctx, data);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &String, env: &Env) {
        self.text_box.lifecycle(ctx, event, data, env);

        if let LifeCycle::FocusChanged(true) = event {
            self.schedule_caret_follow();
            ctx.request_layout();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &String, data: &String, env: &Env) {
        self.text_box.update(ctx, old_data, data, env);
        self.forget_history_if_replaced(data);

        if !old_data.same(data) {
            self.schedule_caret_follow();
            ctx.request_layout();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &String,
        env: &Env,
    ) -> Size {
        let size = self.text_box.layout(ctx, bc, data, env);

        if self.is_caret_follow_due {
            self.is_caret_follow_due = false;
            ctx.submit_command(FOLLOW_CARET.to(ctx.widget_id()));
        }

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &String, env: &Env) {
        self.text_box.paint(ctx, data, env)
    }
}

/// Builds the scrolling diary editor, framed like a text box. The text gets as much room below
/// its last line as the caret keeps below itself, so the last line can scroll up that far too.
pub fn diary_editor() -> impl Widget<String> {
    Scroll::new(DiaryEditor::new())
        .vertical()
        .content_must_fill(true)
        .env_scope(|env, _: &String| {
            let insets = env.get(theme::TEXTBOX_INSETS);
            env.set(
                theme::TEXTBOX_INSETS,
                Insets::new(insets.x0, insets.y0, insets.x1, CARET_MARGIN_BELOW),
            );
            env.set(theme::TEXTBOX_BORDER_WIDTH, 0.0);
        })
        .border(theme::BORDER_DARK, 1.0)
        .rounded(theme::TEXTBOX_BORDER_RADIUS)
        .expand()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_of_an_offset_counts_the_line_breaks_before_it() {
        let text = "first\nsecond\n\nfourth";

        assert_eq!(line_at(text, 0), 0);
        assert_eq!(line_at(text, 5), 0);
        assert_eq!(line_at(text, 6), 1);
        assert_eq!(line_at(text, 14), 3);
        assert_eq!(line_at(text, text.len()), 3);
    }

    #[test]
    fn an_offset_past_the_text_or_inside_a_character_does_not_panic() {
        assert_eq!(line_at("a\nb", 99), 1);
        assert_eq!(line_at("ğ\nx", 1), 1);
    }

    #[test]
    fn a_line_opens_below_the_caret_line_wherever_the_caret_is_on_it() {
        let text = "first\nsecond\nthird";

        assert_eq!(
            with_line_opened_below(text, 8),
            ("first\nsecond\n\nthird".to_string(), 13)
        );
        assert_eq!(
            with_line_opened_below(text, 0),
            ("first\n\nsecond\nthird".to_string(), 6)
        );
    }

    #[test]
    fn a_line_opens_below_the_last_line_at_the_end_of_the_text() {
        assert_eq!(with_line_opened_below("only", 2), ("only\n".to_string(), 5));
        assert_eq!(with_line_opened_below("", 0), ("\n".to_string(), 1));
    }

    #[test]
    fn a_caret_on_the_line_break_opens_the_line_below_that_line() {
        assert_eq!(
            with_line_opened_below("ab\ncd", 2),
            ("ab\n\ncd".to_string(), 3)
        );
    }

    #[test]
    fn a_caret_past_the_text_opens_a_line_at_the_end() {
        assert_eq!(with_line_opened_below("ğ", 99), ("ğ\n".to_string(), 3));
    }
}
