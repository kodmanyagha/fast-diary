pub mod render;

use std::{collections::HashMap, time::Duration};

use druid::{
    piet::InterpolationMode,
    text::RichText,
    widget::{FillStrat, Image, LineBreaking, RawLabel, Scroll},
    BoxConstraints, Env, Event, EventCtx, ImageBuf, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Rect, Size, TimerToken, UpdateCtx, Widget, WidgetExt, WidgetPod,
};

use crate::{modal::app_state::AppState, storage::asset_store::AssetStore};

use render::{markdown_to_rich_text, missing_image_text, render_markdown, PreviewBlock};

const CONTENT_PADDING: (f64, f64) = (12.0, 8.0);

/// Room left between a block of text and an image next to it.
const BLOCK_SPACING: f64 = 8.0;

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

fn diary_key(app_state: &AppState) -> (bool, &str, Option<&str>) {
    (
        app_state.current_diary.is_selected,
        app_state.current_diary.diary.file_name.as_str(),
        app_state.diary_base_path.as_deref(),
    )
}

/// Returns the size that shows an image of `natural_size` at most `max_width` wide, keeping its
/// proportions and never enlarging it.
fn fitted_image_size(natural_size: Size, max_width: f64) -> Size {
    let is_too_wide = max_width.is_finite() && natural_size.width > max_width;

    if is_too_wide && natural_size.width > 0.0 {
        Size::new(
            max_width,
            natural_size.height * max_width / natural_size.width,
        )
    } else {
        natural_size
    }
}

/// Reads and decodes the image at `reference` of the diary folder, or `None` when it cannot be
/// shown.
fn load_image(store: Option<&AssetStore>, reference: &str) -> Option<ImageBuf> {
    store?
        .read(reference)
        .inspect_err(|err| tracing::debug!("Could not read image {reference}: {err}"))
        .ok()
        .and_then(|content| {
            ImageBuf::from_data(&content)
                .inspect_err(|err| tracing::debug!("Could not decode image {reference}: {err}"))
                .ok()
        })
        .filter(|image| image.width() > 0 && image.height() > 0)
}

/// The widget that shows one block of the preview.
enum BlockView {
    Text {
        label: Box<WidgetPod<RichText, RawLabel<RichText>>>,
        text: RichText,
    },
    Image {
        image: Box<WidgetPod<(), Image>>,
        natural_size: Size,
    },
}

impl BlockView {
    fn text(rendered: &render::RenderedMarkdown) -> Self {
        Self::Text {
            label: Box::new(WidgetPod::new(
                RawLabel::new().with_line_break_mode(LineBreaking::WordWrap),
            )),
            text: markdown_to_rich_text(rendered),
        }
    }

    fn image(image: ImageBuf) -> Self {
        let natural_size = image.size();

        Self::Image {
            image: Box::new(WidgetPod::new(
                Image::new(image)
                    .fill_mode(FillStrat::Fill)
                    .interpolation_mode(InterpolationMode::Bilinear),
            )),
            natural_size,
        }
    }

    /// Builds the view of `block`, taking decoded images from `image_cache` and keeping the
    /// newly decoded ones there.
    fn of(
        block: &PreviewBlock,
        image_cache: &mut HashMap<String, ImageBuf>,
        store: Option<&AssetStore>,
    ) -> Self {
        match block {
            PreviewBlock::Text(rendered) => Self::text(rendered),
            PreviewBlock::Image {
                reference,
                description,
            } => {
                let cached = image_cache.get(reference).cloned();
                cached
                    .or_else(|| {
                        load_image(store, reference).inspect(|image| {
                            image_cache.insert(reference.clone(), image.clone());
                        })
                    })
                    .map_or_else(
                        || Self::text(&missing_image_text(reference, description)),
                        Self::image,
                    )
            }
        }
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        match self {
            Self::Text { label, text } => label.event(ctx, event, text, env),
            Self::Image { image, .. } => image.event(ctx, event, &mut (), env),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        match self {
            Self::Text { label, text } => label.lifecycle(ctx, event, text, env),
            Self::Image { image, .. } => image.lifecycle(ctx, event, &(), env),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, env: &Env) {
        match self {
            Self::Text { label, text } => label.update(ctx, text, env),
            Self::Image { image, .. } => image.update(ctx, &(), env),
        }
    }

    /// Lays the block out at most `width` wide with its top at `top`, and returns its height.
    fn layout_at(&mut self, ctx: &mut LayoutCtx, width: f64, top: f64, env: &Env) -> f64 {
        let origin = Point::new(0.0, top);
        let size = match self {
            Self::Text { label, text } => {
                let constraints = BoxConstraints::new(Size::ZERO, Size::new(width, f64::INFINITY));
                let size = label.layout(ctx, &constraints, text, env);
                label.set_origin(ctx, origin);
                size
            }
            Self::Image {
                image,
                natural_size,
            } => {
                let constraints = BoxConstraints::tight(fitted_image_size(*natural_size, width));
                let size = image.layout(ctx, &constraints, &(), env);
                image.set_origin(ctx, origin);
                size
            }
        };
        size.height
    }

    fn paint(&mut self, ctx: &mut PaintCtx, env: &Env) {
        match self {
            Self::Text { label, text } => label.paint(ctx, text, env),
            Self::Image { image, .. } => image.paint(ctx, &(), env),
        }
    }

    fn plain_text(&self) -> Option<String> {
        match self {
            Self::Text { text, .. } => Some(druid::piet::TextStorage::as_str(text).to_string()),
            Self::Image { .. } => None,
        }
    }
}

/// Shows the markdown text of the opened diary as styled text, with the images it refers to
/// drawn between the text. A new diary is shown at once, and the changes of the text that is
/// being edited are shown [`PREVIEW_UPDATE_DELAY`] after the last one, because laying out a long
/// text on every key press is slow.
pub struct MarkdownPreview {
    blocks: Vec<BlockView>,
    rendered_text: String,
    image_cache: HashMap<String, ImageBuf>,
    update_timer: TimerToken,
    is_render_due: bool,
}

impl MarkdownPreview {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            rendered_text: String::new(),
            image_cache: HashMap::new(),
            update_timer: TimerToken::INVALID,
            is_render_due: false,
        }
    }

    /// Returns the plain text that the preview shows right now, with its blocks of text
    /// separated by blank lines.
    pub fn rendered_text(&self) -> &str {
        &self.rendered_text
    }

    /// Returns the timer that makes the preview catch up with the text, if a change is waiting.
    pub fn update_timer(&self) -> TimerToken {
        self.update_timer
    }

    /// Rebuilds the blocks from the text of the opened diary. The children change, so the
    /// caller has to tell druid about it.
    fn render(&mut self, data: &AppState) {
        let store = data.asset_store();
        let image_cache = &mut self.image_cache;
        let blocks = render_markdown(&data.txt_diary)
            .iter()
            .map(|block| BlockView::of(block, image_cache, store.as_ref()))
            .collect::<Vec<_>>();

        self.rendered_text = blocks
            .iter()
            .filter_map(BlockView::plain_text)
            .collect::<Vec<_>>()
            .join("\n\n");
        self.blocks = blocks;
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

        self.blocks
            .iter_mut()
            .for_each(|block| block.event(ctx, event, env));
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppState, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.render(data);
        }

        self.blocks
            .iter_mut()
            .for_each(|block| block.lifecycle(ctx, event, env));
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppState, data: &AppState, env: &Env) {
        let diary_changed = diary_key(old_data) != diary_key(data);
        let action = next_action(
            diary_changed,
            old_data.txt_diary != data.txt_diary,
            self.is_render_due,
        );

        if diary_changed {
            self.image_cache.clear();
        }

        match action {
            PreviewAction::RenderNow => {
                self.render(data);
                ctx.children_changed();
            }
            PreviewAction::RenderAfterDelay => {
                self.update_timer = ctx.request_timer(PREVIEW_UPDATE_DELAY);
            }
            PreviewAction::Nothing => {}
        }

        if action != PreviewAction::RenderNow {
            self.blocks
                .iter_mut()
                .for_each(|block| block.update(ctx, env));
        }

        if old_data.editor_cursor_line != data.editor_cursor_line {
            self.scroll_to_cursor_line(ctx, &data.txt_diary, data.editor_cursor_line);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppState,
        env: &Env,
    ) -> Size {
        let width = bc.max().width;
        let bottom = self.blocks.iter_mut().fold(None, |bottom, block| {
            let top = bottom.map_or(0.0, |bottom: f64| bottom + BLOCK_SPACING);
            Some(top + block.layout_at(ctx, width, top, env))
        });

        let content_width = if width.is_finite() { width } else { 0.0 };
        bc.constrain(Size::new(content_width, bottom.unwrap_or(0.0)))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &AppState, env: &Env) {
        self.blocks
            .iter_mut()
            .for_each(|block| block.paint(ctx, env));
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

    #[test]
    fn a_wide_image_is_shrunk_to_the_width_keeping_its_proportions() {
        assert_eq!(
            fitted_image_size(Size::new(1000.0, 500.0), 400.0),
            Size::new(400.0, 200.0)
        );
    }

    #[test]
    fn a_narrow_image_keeps_its_own_size() {
        assert_eq!(
            fitted_image_size(Size::new(200.0, 100.0), 400.0),
            Size::new(200.0, 100.0)
        );
        assert_eq!(
            fitted_image_size(Size::new(200.0, 100.0), f64::INFINITY),
            Size::new(200.0, 100.0)
        );
    }

    #[test]
    fn an_image_that_cannot_be_read_is_not_loaded() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        std::fs::write(folder.path().join("broken.png"), b"not a picture")?;
        let store = AssetStore::new(folder.path(), crate::storage::codec::Codec::Plain);

        assert!(load_image(Some(&store), "broken.png").is_none());
        assert!(load_image(Some(&store), "missing.png").is_none());
        assert!(load_image(None, "broken.png").is_none());
        Ok(())
    }

    #[test]
    fn a_stored_image_is_decoded() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let png = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99,
            0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        std::fs::write(folder.path().join("dot.png"), png)?;
        let store = AssetStore::new(folder.path(), crate::storage::codec::Codec::Plain);

        let image = load_image(Some(&store), "dot.png");

        assert_eq!(image.map(|image| image.size()), Some(Size::new(1.0, 1.0)));
        Ok(())
    }
}
