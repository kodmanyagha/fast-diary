mod undo_history;

use std::{ops::Range, time::Instant};

use druid::{
    commands,
    text::Selection,
    theme,
    widget::{Scroll, TextBox},
    Application, BoxConstraints, Clipboard, Code, Command, Data, Env, Event, EventCtx, Insets,
    KbKey, KeyEvent, LayoutCtx, LifeCycle, LifeCycleCtx, Modifiers, PaintCtx, Point, Rect,
    Selector, Size, Target, UpdateCtx, Vec2, Widget, WidgetExt,
};

use crate::{
    clipboard::paste_content::{classify_paste, PasteContent},
    consts::druid_selector,
    utils::diary::with_block_inserted,
};

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

/// Returns the largest character boundary of `text` that is not after `offset`.
fn floor_char_boundary(text: &str, offset: usize) -> usize {
    (0..=offset.min(text.len()))
        .rev()
        .find(|boundary| text.is_char_boundary(*boundary))
        .unwrap_or_default()
}

/// Returns the offset where the line that the byte `offset` falls on starts.
fn line_start_at(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |line_break| line_break + 1)
}

/// Returns `text` without the lines that the bytes in `selected` touch, together with the
/// caret offset afterwards: on the line that took their place, at the column the caret was at
/// on its own line, or at the end of that line when it is shorter. A selection that ends right
/// at the start of a line leaves that line alone.
fn with_lines_deleted(text: &str, selected: Range<usize>, caret: usize) -> (String, usize) {
    let start = floor_char_boundary(text, selected.start);
    let end = floor_char_boundary(text, selected.end.max(start));
    let end = if end > start && text[..end].ends_with('\n') {
        end - 1
    } else {
        end
    };
    let first_line_start = line_start_at(text, start);
    let caret = floor_char_boundary(text, caret.clamp(start, end.max(start)));
    let column = caret - line_start_at(text, caret);

    let (removed, next_line_start) = match text[end..].find('\n') {
        Some(distance) => (first_line_start..end + distance + 1, first_line_start),
        None if first_line_start > 0 => {
            let previous_line_start = line_start_at(text, first_line_start - 1);
            (first_line_start - 1..text.len(), previous_line_start)
        }
        None => (0..text.len(), 0),
    };
    let remaining = [&text[..removed.start], &text[removed.end..]].concat();
    let next_line_end = remaining[next_line_start..]
        .find('\n')
        .map_or(remaining.len(), |distance| next_line_start + distance);
    let caret_after =
        floor_char_boundary(&remaining, (next_line_start + column).min(next_line_end));
    (remaining, caret_after)
}

/// Returns `text` with `pasted` in place of the bytes in `selected`, together with the offset
/// right after the pasted text.
fn with_text_pasted(text: &str, selected: Range<usize>, pasted: &str) -> (String, usize) {
    let start = selected.start.min(text.len());
    let end = selected.end.clamp(start, text.len());
    let replaced = [
        text.get(..start).unwrap_or(text),
        pasted,
        text.get(end..).unwrap_or_default(),
    ]
    .concat();
    (replaced, start + pasted.len())
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
    visible_area: Option<Rect>,
}

impl DiaryEditor {
    pub fn new() -> Self {
        Self {
            text_box: TextBox::multiline().with_line_wrapping(true),
            is_caret_follow_due: false,
            last_reported_cursor_line: None,
            undo_history: UndoHistory::default(),
            last_known_state: None,
            visible_area: None,
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
        self.apply_edit(ctx, data, opened, new_line_start);
    }

    /// Puts `block` on a line of its own in place of the selection, moving the caret after it
    /// and recording the change as one undo step. Does nothing while the input method is
    /// composing text.
    fn insert_block(&mut self, ctx: &mut EventCtx, data: &mut String, block: &str) {
        if self.text_box.text().is_composing() {
            return;
        }
        let selection = self.selection_or_end(data);
        let (inserted, caret_after) = with_block_inserted(data, selection.range(), block);
        self.apply_edit(ctx, data, inserted, caret_after);
    }

    /// Deletes the lines that the selection touches, recording the change as one undo step.
    /// Does nothing while the input method is composing text.
    fn delete_selected_lines(&mut self, ctx: &mut EventCtx, data: &mut String) {
        if self.text_box.text().is_composing() || data.is_empty() {
            return;
        }
        let selection = self.selection_or_end(data);
        let (remaining, caret_after) =
            with_lines_deleted(data, selection.range(), selection.active);
        self.apply_edit(ctx, data, remaining, caret_after);
    }

    /// Pastes the text on the clipboard in place of the selection as it is, even when it names
    /// image files, recording the change as one undo step.
    fn paste_plain_text(&mut self, ctx: &mut EventCtx, data: &mut String) {
        if self.text_box.text().is_composing() {
            return;
        }
        let Some(pasted) = Application::try_global()
            .and_then(|application| application.clipboard().get_string())
            .filter(|pasted| !pasted.is_empty())
        else {
            return;
        };
        let selection = self.selection_or_end(data);
        let (replaced, caret_after) = with_text_pasted(data, selection.range(), &pasted);
        self.apply_edit(ctx, data, replaced, caret_after);
    }

    /// Decides what a paste of `clipboard` does. Text is left to the text box, while files,
    /// images and content that cannot be pasted are handed to the window to deal with.
    /// Returns whether the text box still has to paste.
    fn route_paste(ctx: &mut EventCtx, clipboard: &Clipboard) -> bool {
        match classify_paste(clipboard, dirs::home_dir().as_deref()) {
            PasteContent::Text => true,
            PasteContent::Nothing => false,
            content => {
                ctx.submit_command(druid_selector::PASTE_CONTENT.with(content));
                false
            }
        }
    }

    /// Returns the text offset under `window_point`, a point in window coordinates, when that
    /// point lies on the part of the editor that can be seen.
    fn text_position_at(&self, ctx: &EventCtx, window_point: Point, env: &Env) -> Option<usize> {
        let local_point = window_point - ctx.window_origin().to_vec2();
        let editor_area = ctx.size().to_rect();
        let is_visible = self
            .visible_area
            .is_none_or(|visible_area| visible_area.contains(local_point));
        let text = self.text_box.text();

        (editor_area.contains(local_point) && is_visible && text.can_read()).then(|| {
            let insets = env.get(theme::TEXTBOX_INSETS);
            let layout_point = local_point - Vec2::new(insets.x0, insets.y0);
            text.borrow().layout.text_position_for_point(layout_point)
        })
    }

    /// Moves the caret to where files were dropped, so that their images are inserted there,
    /// and focuses the editor. Drops outside the visible editor leave the caret where it is.
    fn place_caret_at_drop(&mut self, ctx: &mut EventCtx, window_point: Point, env: &Env) {
        if self.text_box.text().is_composing() {
            return;
        }
        let Some(position) = self.text_position_at(ctx, window_point, env) else {
            return;
        };
        let text_component = self.text_box.text_mut();
        if text_component.can_write() {
            if let Some(invalidation) = text_component
                .borrow_mut()
                .set_selection(Selection::caret(position))
            {
                ctx.invalidate_text_input(invalidation);
            }
        }
        ctx.request_focus();
        self.schedule_caret_follow();
        ctx.request_layout();
    }

    /// Replaces the text with `edited`, places the caret at `caret` and records the change as
    /// one undo step.
    fn apply_edit(&mut self, ctx: &mut EventCtx, data: &mut String, edited: String, caret: usize) {
        *data = edited;
        let text_component = self.text_box.text_mut();
        if text_component.can_write() {
            if let Some(invalidation) = text_component
                .borrow_mut()
                .set_selection(Selection::caret(caret))
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
            Event::Command(cmd) if cmd.is(druid_selector::EDITOR_INSERT_BLOCK) => {
                let block = cmd.get_unchecked(druid_selector::EDITOR_INSERT_BLOCK);
                self.forget_history_if_replaced(data);
                self.insert_block(ctx, data, block);
                ctx.set_handled();
                self.report_cursor_line(ctx, data);
                return;
            }
            Event::Command(cmd)
                if cmd.is(druid_selector::EDITOR_DELETE_LINE) && ctx.has_focus() =>
            {
                self.forget_history_if_replaced(data);
                self.delete_selected_lines(ctx, data);
                ctx.set_handled();
                self.report_cursor_line(ctx, data);
                return;
            }
            Event::Command(cmd)
                if cmd.is(druid_selector::EDITOR_PASTE_PLAIN) && ctx.has_focus() =>
            {
                self.forget_history_if_replaced(data);
                self.paste_plain_text(ctx, data);
                ctx.set_handled();
                self.report_cursor_line(ctx, data);
                return;
            }
            Event::Command(cmd) if cmd.is(druid_selector::FILES_DROPPED) => {
                let dropped = cmd.get_unchecked(druid_selector::FILES_DROPPED);
                if let Some(window_point) = dropped.window_point {
                    self.forget_history_if_replaced(data);
                    self.place_caret_at_drop(ctx, window_point, env);
                    self.report_cursor_line(ctx, data);
                }
                return;
            }
            Event::Paste(clipboard) if !Self::route_paste(ctx, clipboard) => {
                ctx.set_handled();
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

        match event {
            LifeCycle::FocusChanged(true) => {
                self.schedule_caret_follow();
                ctx.request_layout();
            }
            LifeCycle::ViewContextChanged(view_context) => {
                self.visible_area = Some(view_context.clip);
            }
            _ => {}
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
    fn the_caret_line_is_deleted_and_the_caret_keeps_its_column() {
        assert_eq!(
            with_lines_deleted("first\nsecond\nthird", 9..9, 9),
            ("first\nthird".to_string(), 9)
        );
    }

    #[test]
    fn the_caret_stops_at_the_end_of_a_shorter_next_line() {
        assert_eq!(
            with_lines_deleted("a\nlonger line\nab", 10..10, 10),
            ("a\nab".to_string(), 4)
        );
    }

    #[test]
    fn deleting_the_last_line_moves_the_caret_onto_the_previous_one() {
        assert_eq!(
            with_lines_deleted("first\nsecond\nlast", 16..16, 16),
            ("first\nsecond".to_string(), 9)
        );
        assert_eq!(with_lines_deleted("only line", 4..4, 4), (String::new(), 0));
    }

    #[test]
    fn every_line_the_selection_touches_is_deleted() {
        assert_eq!(
            with_lines_deleted("one\ntwo\nthree\nfour", 5..10, 10),
            ("one\nfour".to_string(), 6)
        );
    }

    #[test]
    fn a_selection_ending_at_a_line_start_spares_that_line() {
        assert_eq!(
            with_lines_deleted("one\ntwo\nthree", 0..4, 4),
            ("two\nthree".to_string(), 3)
        );
    }

    #[test]
    fn empty_lines_and_multibyte_text_are_handled() {
        assert_eq!(
            with_lines_deleted("ğüş\n\nçöı", 7..7, 7),
            ("ğüş\nçöı".to_string(), 7)
        );
        assert_eq!(
            with_lines_deleted("ab\nğüş", 6..6, 6),
            ("ab".to_string(), 2)
        );
    }

    #[test]
    fn pasted_text_replaces_the_selection() {
        assert_eq!(
            with_text_pasted("one two three", 4..7, "2"),
            ("one 2 three".to_string(), 5)
        );
        assert_eq!(
            with_text_pasted("abc", 3..3, "/tmp/a.png"),
            ("abc/tmp/a.png".to_string(), 13)
        );
    }

    #[test]
    fn a_selection_past_the_text_is_kept_inside_it() {
        assert_eq!(with_text_pasted("ab", 5..9, "c"), ("abc".to_string(), 3));
    }

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
