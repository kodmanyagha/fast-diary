use druid::{
    theme,
    widget::{Scroll, TextBox},
    BoxConstraints, Code, Command, Data, Env, Event, EventCtx, Insets, LayoutCtx, LifeCycle,
    LifeCycleCtx, Modifiers, PaintCtx, Rect, Selector, Size, Target, UpdateCtx, Vec2, Widget,
    WidgetExt,
};

use crate::consts::druid_selector;

const FOLLOW_CARET: Selector<()> = Selector::new("diary_editor.follow_caret");

/// Room kept visible above the caret's line when the editor scrolls to it.
const CARET_MARGIN_ABOVE: f64 = 8.0;
/// Room kept visible below the caret's line, so the line being typed never touches the bottom.
const CARET_MARGIN_BELOW: f64 = 28.0;

/// Returns the 0-based line of `text` that the byte `offset` falls on.
fn line_at(text: &str, offset: usize) -> usize {
    text.get(..offset).unwrap_or(text).matches('\n').count()
}

/// A multi-line diary editor that is laid out at the full height of its text inside an outer
/// [`Scroll`], and scrolls that [`Scroll`] itself to keep the caret's line fully in view.
///
/// druid's own [`TextBox`] scrolling measures the caret without the text insets and without any
/// vertical margin, so the line being typed ends up partly hidden under the bottom edge.
pub struct DiaryEditor {
    text_box: TextBox<String>,
    is_caret_follow_due: bool,
    last_reported_cursor_line: Option<usize>,
}

impl DiaryEditor {
    pub fn new() -> Self {
        Self {
            text_box: TextBox::multiline().with_line_wrapping(true),
            is_caret_follow_due: false,
            last_reported_cursor_line: None,
        }
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

        let caret_before = self.caret();
        self.text_box.event(ctx, event, data, env);
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
}
