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
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RenderedMarkdown {
    pub text: String,
    pub spans: Vec<StyledSpan>,
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
}

impl Renderer {
    fn accept(mut self, event: Event<'_>) -> Self {
        match event {
            Event::Start(tag) => self.start(&tag),
            Event::End(tag_end) => self.end(&tag_end),
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                self.text.push_str(&text)
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

    fn finish(mut self) -> RenderedMarkdown {
        self.text.truncate(self.text.trim_end().len());
        let text_length = self.text.len();

        let mut spans = self
            .spans
            .into_iter()
            .map(|span| StyledSpan {
                range: span.range.start.min(text_length)..span.range.end.min(text_length),
                style: span.style,
            })
            .filter(|span| !span.range.is_empty())
            .collect::<Vec<_>>();
        spans.sort_by_key(|span| (span.range.start, std::cmp::Reverse(span.range.end)));

        RenderedMarkdown {
            text: self.text,
            spans,
        }
    }
}

fn span_style(tag: &Tag<'_>) -> Option<SpanStyle> {
    match tag {
        Tag::Heading { level, .. } => Some(SpanStyle::Heading(*level)),
        Tag::BlockQuote(_) => Some(SpanStyle::BlockQuote),
        Tag::CodeBlock(_) => Some(SpanStyle::CodeBlock),
        Tag::Emphasis => Some(SpanStyle::Emphasis),
        Tag::Strong => Some(SpanStyle::Strong),
        Tag::Strikethrough => Some(SpanStyle::Strikethrough),
        Tag::Link { .. } | Tag::Image { .. } => Some(SpanStyle::Link),
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
            | TagEnd::Image
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

/// Parses `source` as markdown into plain text plus the ranges that need styling.
pub fn render_markdown(source: &str) -> RenderedMarkdown {
    Parser::new_ext(
        source,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    )
    .fold(Renderer::default(), Renderer::accept)
    .finish()
}

/// Converts `source` from markdown into styled text that a label can display.
pub fn markdown_to_rich_text(source: &str) -> RichText {
    let rendered = render_markdown(source);
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

    fn text_of(source: &str) -> String {
        render_markdown(source).text
    }

    fn styles_over(source: &str) -> Vec<(String, SpanStyle)> {
        let rendered = render_markdown(source);

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
        assert_eq!(render_markdown(""), RenderedMarkdown::default());
        assert_eq!(render_markdown("\n\n  \n"), RenderedMarkdown::default());
    }

    #[test]
    fn unterminated_markup_is_shown_as_typed() {
        assert_eq!(text_of("half **bold"), "half **bold");
    }

    #[test]
    fn rich_text_holds_the_rendered_text() {
        assert_eq!(markdown_to_rich_text("# Hi\n\n- a").as_str(), "Hi\n\n• a");
    }
}
