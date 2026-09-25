pub mod render;

use std::time::Duration;

use druid::{
    piet::TextStorage,
    text::RichText,
    widget::{LineBreaking, RawLabel, Scroll},
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Rect, Size,
    TimerToken, UpdateCtx, Widget, WidgetExt,
};

use crate::modal::app_state::AppState;

use render::markdown_to_rich_text;

const CONTENT_PADDING: (f64, f64) = (12.0, 8.0);

/// How long the text has to stay unchanged before the preview shows it.
pub const PREVIEW_UPDATE_DELAY: Duration = Duration::from_millis(600);

/// Vertical slack kept around the caret's estimated position when scrolling the preview to
/// follow it, so the targeted line lands away from the very edge of the viewport.
const CURSOR_FOLLOW_MARGIN: f64 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewAction {
    RenderNow,
    RenderAfterDelay,
    Nothing,
}

/// Decides what the preview does when the data changed. Another diary is shown at once, while
/// the edits of the same diary wait until the text has been left alone for a while.
fn next_action(diary_changed: bool, text_changed: bool, is_render_due: bool) -> PreviewAction {
    if diary_changed || is_render_due {
        PreviewAction::RenderNow
    } else if text_changed {
        PreviewAction::RenderAfterDelay
    } else {
        PreviewAction::Nothing
    }
}

fn diary_key(app_state: &AppState) -> (bool, &str) {
    (
        app_state.current_diary.is_selected,
        app_state.current_diary.diary.file_name.as_str(),
    )
}

/// Shows the markdown text of the opened diary as styled text. A new diary is shown at once, and
/// the changes of the text that is being edited are shown [`PREVIEW_UPDATE_DELAY`] after the last
/// one, because laying out a long text on every key press is slow.
pub struct MarkdownPreview {
    label: RawLabel<RichText>,
    rendered: RichText,
    update_timer: TimerToken,
    is_render_due: bool,
}

impl MarkdownPreview {
    pub fn new() -> Self {
        Self {
            label: RawLabel::new().with_line_break_mode(LineBreaking::WordWrap),
            rendered: RichText::new("".into()),
            update_timer: TimerToken::INVALID,
            is_render_due: false,
        }
    }

    /// Returns the plain text that the preview shows right now.
    pub fn rendered_text(&self) -> &str {
        self.rendered.as_str()
    }

    /// Returns the timer that makes the preview catch up with the text, if a change is waiting.
    pub fn update_timer(&self) -> TimerToken {
        self.update_timer
    }

    fn render(&mut self, markdown: &str) {
        self.rendered = markdown_to_rich_text(markdown);
        self.is_render_due = false;
        self.update_timer = TimerToken::INVALID;
    }

    /// Scrolls the enclosing [`Scroll`] so that the area matching the editor's caret line stays
    /// in view. The source text has no exact line-by-line correspondence with the rendered rich
    /// text, so the caret's line is only approximated by its ratio through the source text,
    /// mapped onto the rendered content's height.
    fn scroll_to_cursor_line(&self, ctx: &mut UpdateCtx, source_text: &str, cursor_line: usize) {
        let last_line = source_text.lines().count().saturating_sub(1).max(1);
        let ratio = cursor_line.min(last_line) as f64 / last_line as f64;
        let target_y = ratio * ctx.size().height;

        ctx.scroll_area_to_view(Rect::new(
            0.0,
            (target_y - CURSOR_FOLLOW_MARGIN).max(0.0),
            1.0,
            target_y + CURSOR_FOLLOW_MARGIN,
        ));
    }
}

impl Default for MarkdownPreview {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget<AppState> for MarkdownPreview {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut AppState, env: &Env) {
        if let Event::Timer(token) = event {
            if *token == self.update_timer {
                self.is_render_due = true;
                ctx.request_update();
                ctx.set_handled();
                return;
            }
        }

        self.label.event(ctx, event, &mut self.rendered, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppState, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.render(&data.txt_diary);
        }

        self.label.lifecycle(ctx, event, &self.rendered, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppState, data: &AppState, env: &Env) {
        let previous = self.rendered.clone();
        let action = next_action(
            diary_key(old_data) != diary_key(data),
            old_data.txt_diary != data.txt_diary,
            self.is_render_due,
        );

        match action {
            PreviewAction::RenderNow => self.render(&data.txt_diary),
            PreviewAction::RenderAfterDelay => {
                self.update_timer = ctx.request_timer(PREVIEW_UPDATE_DELAY);
            }
            PreviewAction::Nothing => {}
        }

        if old_data.editor_cursor_line != data.editor_cursor_line {
            self.scroll_to_cursor_line(ctx, &data.txt_diary, data.editor_cursor_line);
        }

        self.label.update(ctx, &previous, &self.rendered, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppState,
        env: &Env,
    ) -> Size {
        self.label.layout(ctx, bc, &self.rendered, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &AppState, env: &Env) {
        self.label.paint(ctx, &self.rendered, env)
    }
}

/// Builds a vertically scrolling view that displays the markdown text of the opened diary.
pub fn markdown_preview() -> impl Widget<AppState> {
    Scroll::new(
        MarkdownPreview::new()
            .padding(CONTENT_PADDING)
            .expand_width(),
    )
    .vertical()
    .expand()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn another_diary_is_shown_at_once() {
        assert_eq!(next_action(true, true, false), PreviewAction::RenderNow);
        assert_eq!(next_action(true, false, false), PreviewAction::RenderNow);
    }

    #[test]
    fn edits_of_the_same_diary_wait() {
        assert_eq!(
            next_action(false, true, false),
            PreviewAction::RenderAfterDelay
        );
    }

    #[test]
    fn a_render_that_is_due_is_done_even_when_nothing_else_changed() {
        assert_eq!(next_action(false, false, true), PreviewAction::RenderNow);
        assert_eq!(next_action(false, true, true), PreviewAction::RenderNow);
    }

    #[test]
    fn unrelated_changes_do_nothing() {
        assert_eq!(next_action(false, false, false), PreviewAction::Nothing);
    }
}
