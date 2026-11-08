use std::ops::Range;

use druid::{
    text::{AttributesAdder, RichText, RichTextBuilder},
    Color, FontFamily, FontStyle, FontWeight,
};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

const BULLET_MARKER: &str = "• ";
const CHECKED_MARKER: &str = "☑ ";
const UNCHECKED_MARKER: &str = "☐ ";
const RULE_TEXT: &str = "────────────────────";
const NESTED_LIST_INDENT: &str = "  ";

const QUOTE_COLOR: Color = Color::grey8(0x9a);
const CODE_COLOR: Color = Color::rgb8(0xc8, 0xa2, 0x76);
const LINK_COLOR: Color = Color::rgb8(0x6c, 0xa6, 0xff);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStyle {
    Heading(HeadingLevel),
    BlockQuote,
    CodeBlock,
    InlineCode,
    Emphasis,
    Strong,
    Strikethrough,
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub range: Range<usize>,
    pub style: SpanStyle,
}

/// Plain text of a rendered markdown document together with the styled ranges inside it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RenderedMarkdown {
    pub text: String,
    pub spans: Vec<StyledSpan>,
}

/// One piece of a rendered markdown document, shown below the piece before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewBlock {
    Text(RenderedMarkdown),
    Image {
        reference: String,
        description: String,
    },
}

/// The image whose description is being read, until its end tag is reached.
#[derive(Debug, Default)]
struct PendingImage {
    reference: String,
    description: String,
}

#[derive(Debug, Clone, Copy)]
enum ListKind {
    Bullet,
    Numbered { next_number: u64 },
}

#[derive(Default)]
struct Renderer {
    text: String,
    spans: Vec<StyledSpan>,
    open_spans: Vec<(usize, SpanStyle)>,
    lists: Vec<ListKind>,
    blocks: Vec<PreviewBlock>,
    pending_image: Option<PendingImage>,
    is_right_after_image: bool,
}

impl Renderer {
    fn accept(mut self, event: Event<'_>) -> Self {
        if let Some(pending_image) = self.pending_image.as_mut() {
            match event {
                Event::End(TagEnd::Image) => self.finish_image(),
                Event::Text(text) | Event::Code(text) => pending_image.description.push_str(&text),
                _ => {}
            }
            return self;
        }

        match event {
            Event::Start(Tag::Image { dest_url, .. }) => self.start_image(&dest_url),
            Event::Start(tag) => self.start(&tag),
            Event::End(tag_end) => self.end(&tag_end),
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                self.push_text(&text)
            }
            Event::Code(code) => self.push_styled(&code, SpanStyle::InlineCode),
            Event::SoftBreak | Event::HardBreak => self.text.push('\n'),
            Event::Rule => {
                self.text.push_str(RULE_TEXT);
                self.end_block();
            }
            Event::TaskListMarker(is_checked) => self.mark_task(is_checked),
            _ => {}
        }
        self
    }

    fn start(&mut self, tag: &Tag<'_>) {
        match tag {
            Tag::List(first_number) => self.start_list(*first_number),
            Tag::Item => self.start_item(),
            other => {
                if let Some(style) = span_style(other) {
                    self.open_spans.push((self.text.len(), style));
                }
            }
        }
    }

    fn end(&mut self, tag_end: &TagEnd) {
        if closes_span(tag_end) {
            self.close_span();
        }

        match tag_end {
            TagEnd::Item => self.ensure_trailing_newlines(1),
            TagEnd::List(_) => {
                self.lists.pop();
                if self.lists.is_empty() {
                    self.ensure_trailing_newlines(2);
                }
            }
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::HtmlBlock => self.end_block(),
            _ => {}
        }
    }

    /// Adds `text` to the current block. Text that goes on right after an image starts a new
    /// block, so the space that separated it from the image is dropped.
    fn push_text(&mut self, text: &str) {
        let continues_after_image = self.is_right_after_image && self.text.is_empty();
        self.text.push_str(if continues_after_image {
            text.trim_start()
        } else {
            text
        });
        self.is_right_after_image = false;
    }

    /// Ends the text read so far as a block of its own, so that the image starting here is
    /// shown below it. Styles that are still open go on in the text after the image.
    fn start_image(&mut self, reference: &str) {
        self.flush_text();
        self.pending_image = Some(PendingImage {
            reference: reference.to_string(),
            description: String::new(),
        });
    }

    fn finish_image(&mut self) {
        if let Some(PendingImage {
            reference,
            description,
        }) = self.pending_image.take()
        {
            self.blocks.push(PreviewBlock::Image {
                reference,
                description,
            });
            self.is_right_after_image = true;
        }
    }

    /// Moves the text read so far into a block, unless it is blank.
    fn flush_text(&mut self) {
        let text_length = self.text.len();
        let still_open = self.open_spans.clone();
        still_open.iter().for_each(|(start, style)| {
            self.spans.push(StyledSpan {
                range: *start..text_length,
                style: *style,
            })
        });
        self.open_spans = still_open
            .into_iter()
            .map(|(_, style)| (0, style))
            .collect();

        let flushed = finish_text(
            std::mem::take(&mut self.text),
            std::mem::take(&mut self.spans),
        );
        if !flushed.text.is_empty() {
            self.blocks.push(PreviewBlock::Text(flushed));
        }
    }

    fn start_list(&mut self, first_number: Option<u64>) {
        if !self.lists.is_empty() {
            self.ensure_trailing_newlines(1);
        }
        self.lists.push(
            first_number.map_or(ListKind::Bullet, |next_number| ListKind::Numbered {
                next_number,
            }),
        );
    }

    fn start_item(&mut self) {
        self.ensure_trailing_newlines(1);

        let indent = NESTED_LIST_INDENT.repeat(self.lists.len().saturating_sub(1));
        let marker = match self.lists.last_mut() {
            Some(ListKind::Numbered { next_number }) => {
                let marker = format!("{next_number}. ");
                *next_number = next_number.saturating_add(1);
                marker
            }
            _ => BULLET_MARKER.to_string(),
        };

        self.text.push_str(&indent);
        self.text.push_str(&marker);
    }

    fn mark_task(&mut self, is_checked: bool) {
        if let Some(length_without_bullet) = self
            .text
            .strip_suffix(BULLET_MARKER)
            .map(|without_bullet| without_bullet.len())
        {
            self.text.truncate(length_without_bullet);
        }

        self.text.push_str(if is_checked {
            CHECKED_MARKER
        } else {
            UNCHECKED_MARKER
        });
    }

    fn push_styled(&mut self, text: &str, style: SpanStyle) {
        let start = self.text.len();
        self.text.push_str(text);
        self.spans.push(StyledSpan {
            range: start..self.text.len(),
            style,
        });
    }

    fn close_span(&mut self) {
        if let Some((start, style)) = self.open_spans.pop() {
            self.spans.push(StyledSpan {
                range: start..self.text.len(),
                style,
            });
        }
    }

    fn end_block(&mut self) {
        let separating_newlines = if self.lists.is_empty() { 2 } else { 1 };
        self.ensure_trailing_newlines(separating_newlines);
    }

    fn ensure_trailing_newlines(&mut self, wanted: usize) {
        if self.text.is_empty() {
            return;
        }

        let existing = self.text.chars().rev().take_while(|c| *c == '\n').count();
        self.text
            .push_str(&"\n".repeat(wanted.saturating_sub(existing)));
    }

    fn finish(mut self) -> Vec<PreviewBlock> {
        self.finish_image();
        self.open_spans.clear();
        self.flush_text();
        self.blocks
    }
}

/// Trims the blank lines around `text` and fits `spans` into what is left.
fn finish_text(text: String, spans: Vec<StyledSpan>) -> RenderedMarkdown {
    let leading_newlines = text.len() - text.trim_start_matches('\n').len();
    let kept = text
        .get(leading_newlines..)
        .map(str::trim_end)
        .unwrap_or_default()
        .to_string();
    let kept_length = kept.len();

    let mut spans = spans
        .into_iter()
        .map(|span| {
            let shift = |offset: usize| offset.saturating_sub(leading_newlines).min(kept_length);
            StyledSpan {
                range: shift(span.range.start)..shift(span.range.end),
                style: span.style,
            }
        })
        .filter(|span| !span.range.is_empty())
        .collect::<Vec<_>>();
    spans.sort_by_key(|span| (span.range.start, std::cmp::Reverse(span.range.end)));

    RenderedMarkdown { text: kept, spans }
}

fn span_style(tag: &Tag<'_>) -> Option<SpanStyle> {
    match tag {
        Tag::Heading { level, .. } => Some(SpanStyle::Heading(*level)),
        Tag::BlockQuote(_) => Some(SpanStyle::BlockQuote),
        Tag::CodeBlock(_) => Some(SpanStyle::CodeBlock),
        Tag::Emphasis => Some(SpanStyle::Emphasis),
        Tag::Strong => Some(SpanStyle::Strong),
        Tag::Strikethrough => Some(SpanStyle::Strikethrough),
        Tag::Link { .. } => Some(SpanStyle::Link),
        _ => None,
    }
}

fn closes_span(tag_end: &TagEnd) -> bool {
    matches!(
        tag_end,
        TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Link
    )
}

fn heading_font_size(level: HeadingLevel) -> f64 {
    match level {
        HeadingLevel::H1 => 28.0,
        HeadingLevel::H2 => 24.0,
        HeadingLevel::H3 => 21.0,
        HeadingLevel::H4 => 18.0,
        HeadingLevel::H5 => 16.0,
        HeadingLevel::H6 => 15.0,
    }
}

fn apply_style(style: SpanStyle, attributes: &mut AttributesAdder<'_>) {
    match style {
        SpanStyle::Heading(level) => {
            attributes
                .size(heading_font_size(level))
                .weight(FontWeight::BOLD);
        }
        SpanStyle::BlockQuote => {
            attributes.style(FontStyle::Italic).text_color(QUOTE_COLOR);
        }
        SpanStyle::CodeBlock | SpanStyle::InlineCode => {
            attributes
                .font_family(FontFamily::MONOSPACE)
                .text_color(CODE_COLOR);
        }
        SpanStyle::Emphasis => {
            attributes.style(FontStyle::Italic);
        }
        SpanStyle::Strong => {
            attributes.weight(FontWeight::BOLD);
        }
        SpanStyle::Strikethrough => {
            attributes.strikethrough(true);
        }
        SpanStyle::Link => {
            attributes.underline(true).text_color(LINK_COLOR);
        }
    }
}

/// Parses `source` as markdown into blocks of plain text, with the ranges that need styling,
/// and the images between them.
pub fn render_markdown(source: &str) -> Vec<PreviewBlock> {
    Parser::new_ext(
        source,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    )
    .fold(Renderer::default(), Renderer::accept)
    .finish()
}

/// Builds the line that stands in for an image that cannot be shown.
pub fn missing_image_text(reference: &str, description: &str) -> RenderedMarkdown {
    let text = match description.trim() {
        "" => format!("[{reference}]"),
        description => format!("[{description}: {reference}]"),
    };
    let spans = vec![StyledSpan {
        range: 0..text.len(),
        style: SpanStyle::BlockQuote,
    }];

    RenderedMarkdown { text, spans }
}

/// Converts a rendered block of text into styled text that a label can display.
pub fn markdown_to_rich_text(rendered: &RenderedMarkdown) -> RichText {
    let mut builder = RichTextBuilder::new();

    builder.push(&rendered.text);
    rendered.spans.iter().for_each(|span| {
        apply_style(
            span.style,
            &mut builder.add_attributes_for_range(span.range.clone()),
        )
    });

    builder.build()
}

#[cfg(test)]
mod tests {
    use druid::piet::TextStorage;

    use super::*;

    fn text_block_of(source: &str) -> RenderedMarkdown {
        match render_markdown(source).as_slice() {
            [PreviewBlock::Text(rendered)] => rendered.clone(),
            _ => RenderedMarkdown::default(),
        }
    }

    fn text_of(source: &str) -> String {
        text_block_of(source).text
    }

    fn styles_over(source: &str) -> Vec<(String, SpanStyle)> {
        let rendered = text_block_of(source);

        rendered
            .spans
            .iter()
            .filter_map(|span| {
                rendered
                    .text
                    .get(span.range.clone())
                    .map(|covered| (covered.to_string(), span.style))
            })
            .collect()
    }

    #[test]
    fn paragraphs_are_separated_by_one_blank_line() {
        assert_eq!(text_of("first\n\nsecond"), "first\n\nsecond");
    }

    #[test]
    fn single_line_breaks_are_kept() {
        assert_eq!(text_of("first\nsecond"), "first\nsecond");
    }

    #[test]
    fn markup_characters_are_dropped() {
        assert_eq!(
            text_of("# Title\n\nsome **bold** text"),
            "Title\n\nsome bold text"
        );
    }

    #[test]
    fn inline_styles_cover_exactly_their_text() {
        assert_eq!(
            styles_over("a *b* **c** ~~d~~ `e` [f](https://example.com)"),
            vec![
                ("b".to_string(), SpanStyle::Emphasis),
                ("c".to_string(), SpanStyle::Strong),
                ("d".to_string(), SpanStyle::Strikethrough),
                ("e".to_string(), SpanStyle::InlineCode),
                ("f".to_string(), SpanStyle::Link),
            ]
        );
    }

    #[test]
    fn headings_carry_their_level() {
        assert_eq!(
            styles_over("## Second level"),
            vec![(
                "Second level".to_string(),
                SpanStyle::Heading(HeadingLevel::H2)
            )]
        );
    }

    #[test]
    fn outer_spans_come_before_the_spans_nested_in_them() {
        assert_eq!(
            styles_over("# Title with `code`"),
            vec![
                (
                    "Title with code".to_string(),
                    SpanStyle::Heading(HeadingLevel::H1)
                ),
                ("code".to_string(), SpanStyle::InlineCode),
            ]
        );
    }

    #[test]
    fn bullet_lists_get_one_marker_per_item() {
        assert_eq!(text_of("- one\n- two\n- three"), "• one\n• two\n• three");
    }

    #[test]
    fn ordered_lists_are_numbered_from_their_first_number() {
        assert_eq!(text_of("3. three\n4. four"), "3. three\n4. four");
    }

    #[test]
    fn nested_lists_are_indented() {
        assert_eq!(
            text_of("- one\n  - inner\n- two"),
            "• one\n  • inner\n• two"
        );
    }

    #[test]
    fn loose_list_items_do_not_get_blank_lines() {
        assert_eq!(text_of("- one\n\n- two"), "• one\n• two");
    }

    #[test]
    fn task_list_items_show_a_check_box_instead_of_a_bullet() {
        assert_eq!(text_of("- [x] done\n- [ ] open"), "☑ done\n☐ open");
    }

    #[test]
    fn a_list_is_separated_from_the_paragraph_after_it() {
        assert_eq!(text_of("- one\n\nafter"), "• one\n\nafter");
    }

    #[test]
    fn horizontal_rules_become_a_line() {
        assert_eq!(
            text_of("above\n\n---\n\nbelow"),
            format!("above\n\n{RULE_TEXT}\n\nbelow")
        );
    }

    #[test]
    fn code_blocks_keep_their_lines_and_are_styled() {
        assert_eq!(
            styles_over("```\nlet a = 1;\nlet b = 2;\n```"),
            vec![("let a = 1;\nlet b = 2;".to_string(), SpanStyle::CodeBlock)]
        );
    }

    #[test]
    fn empty_and_blank_sources_render_nothing() {
        assert_eq!(render_markdown(""), Vec::new());
        assert_eq!(render_markdown("\n\n  \n"), Vec::new());
    }

    #[test]
    fn unterminated_markup_is_shown_as_typed() {
        assert_eq!(text_of("half **bold"), "half **bold");
    }

    #[test]
    fn rich_text_holds_the_rendered_text() {
        assert_eq!(
            markdown_to_rich_text(&text_block_of("# Hi\n\n- a")).as_str(),
            "Hi\n\n• a"
        );
    }

    fn image(reference: &str, description: &str) -> PreviewBlock {
        PreviewBlock::Image {
            reference: reference.to_string(),
            description: description.to_string(),
        }
    }

    fn text_block(text: &str) -> PreviewBlock {
        PreviewBlock::Text(RenderedMarkdown {
            text: text.to_string(),
            spans: Vec::new(),
        })
    }

    #[test]
    fn an_image_on_its_own_line_splits_the_text_around_it() {
        assert_eq!(
            render_markdown("before\n![Sea view](assets/1/sea.png)\nafter"),
            vec![
                text_block("before"),
                image("assets/1/sea.png", "Sea view"),
                text_block("after"),
            ]
        );
    }

    #[test]
    fn an_image_alone_is_a_single_block() {
        assert_eq!(render_markdown("![](a.png)"), vec![image("a.png", "")]);
    }

    #[test]
    fn styled_text_around_an_image_keeps_its_style() {
        let blocks = render_markdown("**bold ![x](a.png) still bold**");

        let styles = blocks
            .iter()
            .filter_map(|block| match block {
                PreviewBlock::Text(rendered) => Some(
                    rendered
                        .spans
                        .iter()
                        .map(|span| (rendered.text[span.range.clone()].to_string(), span.style))
                        .collect::<Vec<_>>(),
                ),
                PreviewBlock::Image { .. } => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            styles,
            vec![
                vec![("bold".to_string(), SpanStyle::Strong)],
                vec![("still bold".to_string(), SpanStyle::Strong)],
            ]
        );
    }

    #[test]
    fn a_missing_image_shows_its_description_and_reference() {
        assert_eq!(missing_image_text("a.png", " Sea ").text, "[Sea: a.png]");
        assert_eq!(missing_image_text("a.png", "").text, "[a.png]");
    }
}
