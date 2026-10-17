use std::time::Duration;

use druid::{
    keyboard_types::Key, theme, Application, BoxConstraints, Color, Data, Env, Event, EventCtx,
    LayoutCtx, LifeCycle, LifeCycleCtx, Modifiers, PaintCtx, Point, RenderContext, Selector, Size,
    TextLayout, TimerToken, UpdateCtx, Widget, WidgetExt,
};

const MASK_CHARACTER: &str = "•";
const BORDER_COLOR: Color = Color::grey8(128);
const FOCUS_COLOR: Color = Color::rgb8(70, 130, 180);
const TEXT_PADDING_X: f64 = 8.0;
const TEXT_PADDING_Y: f64 = 6.0;
const CARET_WIDTH: f64 = 1.0;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaretMove {
    Left,
    Right,
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PasswordEdit {
    Insert(String),
    DeleteBackward,
    DeleteForward,
    Clear,
    Paste,
    MoveCaret(CaretMove),
    Submit,
    Ignore,
}

/// Builds a single line input that shows one bullet per typed character instead of the text,
/// with a caret that can be moved through the hidden text. Pressing Enter submits `on_enter`.
pub fn password_box(on_enter: Selector<()>) -> impl Widget<String> {
    PasswordBox::new(on_enter).expand_width()
}

struct PasswordBox {
    on_enter: Selector<()>,
    mask: TextLayout<String>,
    caret: usize,
    is_caret_shown: bool,
    blink_timer: TimerToken,
    scroll_offset: f64,
}

impl PasswordBox {
    /// Creates an empty input that submits `on_enter` when Enter is pressed.
    fn new(on_enter: Selector<()>) -> Self {
        Self {
            on_enter,
            mask: TextLayout::new(),
            caret: 0,
            is_caret_shown: true,
            blink_timer: TimerToken::INVALID,
            scroll_offset: 0.0,
        }
    }

    /// Shows the caret and makes it blink again from now on, driven by `blink_timer`.
    fn restart_blink(&mut self, blink_timer: TimerToken) {
        self.is_caret_shown = true;
        self.blink_timer = blink_timer;
    }

    /// Returns the byte offset in the bullets that matches the caret.
    fn caret_offset_in_mask(&self) -> usize {
        self.caret * MASK_CHARACTER.len()
    }

    /// Returns the character index that is closest to `point`, given in this widget's
    /// coordinates.
    fn caret_for_point(&self, point: Point) -> usize {
        let point_in_text = Point::new(
            point.x - TEXT_PADDING_X + self.scroll_offset,
            point.y - TEXT_PADDING_Y,
        );
        self.mask.text_position_for_point(point_in_text) / MASK_CHARACTER.len()
    }

    /// Applies the key press to the typed password and the caret.
    fn handle_key(
        &mut self,
        ctx: &mut EventCtx,
        password: &mut String,
        key: &Key,
        mods: Modifiers,
    ) {
        match interpret_key(key, mods) {
            PasswordEdit::Ignore => return,
            PasswordEdit::Submit => ctx.submit_command(self.on_enter),
            PasswordEdit::Paste => {
                self.caret = edit_password(
                    password,
                    self.caret,
                    &PasswordEdit::Insert(clipboard_text()),
                );
            }
            edit => self.caret = edit_password(password, self.caret, &edit),
        }

        self.restart_blink(ctx.request_timer(CARET_BLINK_INTERVAL));
        ctx.request_paint();
        ctx.set_handled();
    }
}

impl Widget<String> for PasswordBox {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, password: &mut String, _env: &Env) {
        match event {
            Event::MouseDown(mouse) => {
                ctx.request_focus();
                self.caret = self.caret_for_point(mouse.pos);
                self.restart_blink(ctx.request_timer(CARET_BLINK_INTERVAL));
                ctx.request_paint();
            }
            Event::KeyDown(key_event) if ctx.is_focused() => {
                self.handle_key(ctx, password, &key_event.key, key_event.mods);
            }
            Event::Timer(token) if *token == self.blink_timer => {
                self.is_caret_shown = !self.is_caret_shown;
                self.blink_timer = ctx.request_timer(CARET_BLINK_INTERVAL);
                ctx.request_paint();
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        password: &String,
        _env: &Env,
    ) {
        match event {
            LifeCycle::WidgetAdded => {
                self.caret = password.chars().count();
                self.mask.set_text(mask_for(password));
            }
            LifeCycle::BuildFocusChain => ctx.register_for_focus(),
            LifeCycle::FocusChanged(true) => {
                self.restart_blink(ctx.request_timer(CARET_BLINK_INTERVAL));
                ctx.request_paint();
            }
            LifeCycle::FocusChanged(false) => {
                self.blink_timer = TimerToken::INVALID;
                ctx.request_paint();
            }
            _ => {}
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_password: &String,
        password: &String,
        _env: &Env,
    ) {
        if !old_password.same(password) {
            self.caret = self.caret.min(password.chars().count());
            self.mask.set_text(mask_for(password));
            ctx.request_layout();
        }

        if self.mask.needs_rebuild_after_update(ctx) {
            ctx.request_layout();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _password: &String,
        env: &Env,
    ) -> Size {
        self.mask.rebuild_if_needed(ctx.text(), env);

        let text_size = self.mask.size();
        let width = if bc.is_width_bounded() {
            bc.max().width
        } else {
            text_size.width + 2.0 * TEXT_PADDING_X
        };

        bc.constrain(Size::new(width, text_size.height + 2.0 * TEXT_PADDING_Y))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _password: &String, env: &Env) {
        let size = ctx.size();
        let caret_line = self
            .mask
            .cursor_line_for_text_position(self.caret_offset_in_mask());
        self.scroll_offset = scroll_offset_keeping_caret_visible(
            self.scroll_offset,
            caret_line.p0.x,
            size.width - 2.0 * TEXT_PADDING_X,
        );

        let text_origin = Point::new(TEXT_PADDING_X - self.scroll_offset, TEXT_PADDING_Y);
        let text_area = size.to_rect().inset((CARET_WIDTH - TEXT_PADDING_X, 0.0));
        let is_caret_drawn = ctx.is_focused() && self.is_caret_shown;
        let caret_color = env.get(theme::CURSOR_COLOR);

        ctx.with_save(|ctx| {
            ctx.clip(text_area);
            self.mask.draw(ctx, text_origin);
            if is_caret_drawn {
                ctx.stroke(
                    caret_line + text_origin.to_vec2(),
                    &caret_color,
                    CARET_WIDTH,
                );
            }
        });

        let (border_color, border_width) = if ctx.has_focus() {
            (FOCUS_COLOR, 2.0)
        } else {
            (BORDER_COLOR, 1.0)
        };
        ctx.stroke(
            size.to_rect().inset(-border_width / 2.0),
            &border_color,
            border_width,
        );
    }
}

/// Returns one bullet for each character of `password`.
fn mask_for(password: &str) -> String {
    MASK_CHARACTER.repeat(password.chars().count())
}

/// Returns the byte offset of the character index `caret` in `password`, or the end of the
/// password when the index is past its last character.
fn byte_offset(password: &str, caret: usize) -> usize {
    password
        .char_indices()
        .nth(caret)
        .map_or(password.len(), |(offset, _)| offset)
}

/// Applies `edit` to `password` with the caret at the character index `caret`, and returns the
/// character index of the caret afterwards.
fn edit_password(password: &mut String, caret: usize, edit: &PasswordEdit) -> usize {
    let char_count = password.chars().count();
    let caret = caret.min(char_count);

    match edit {
        PasswordEdit::Insert(text) => {
            password.insert_str(byte_offset(password, caret), text);
            caret + text.chars().count()
        }
        PasswordEdit::DeleteBackward if caret > 0 => {
            password.remove(byte_offset(password, caret - 1));
            caret - 1
        }
        PasswordEdit::DeleteForward if caret < char_count => {
            password.remove(byte_offset(password, caret));
            caret
        }
        PasswordEdit::Clear => {
            password.clear();
            0
        }
        PasswordEdit::MoveCaret(CaretMove::Left) => caret.saturating_sub(1),
        PasswordEdit::MoveCaret(CaretMove::Right) => (caret + 1).min(char_count),
        PasswordEdit::MoveCaret(CaretMove::Start) => 0,
        PasswordEdit::MoveCaret(CaretMove::End) => char_count,
        PasswordEdit::DeleteBackward
        | PasswordEdit::DeleteForward
        | PasswordEdit::Paste
        | PasswordEdit::Submit
        | PasswordEdit::Ignore => caret,
    }
}

/// Returns how far the bullets are scrolled to the left so that the caret at `caret_x` stays
/// inside the `visible_width` of the input, moving the current offset as little as possible.
fn scroll_offset_keeping_caret_visible(
    current_offset: f64,
    caret_x: f64,
    visible_width: f64,
) -> f64 {
    let caret_x = caret_x.max(0.0);
    let lowest_offset = (caret_x - visible_width.max(0.0)).max(0.0);

    current_offset.clamp(lowest_offset, caret_x)
}

/// Translates a key press into an edit of the password. Ctrl+Alt is treated as AltGr, so
/// characters typed with AltGr are inserted like any other. The hidden text has no visible
/// words, so Ctrl with an arrow jumps to the start or the end.
fn interpret_key(key: &Key, modifiers: Modifiers) -> PasswordEdit {
    let is_shortcut = modifiers.contains(Modifiers::CONTROL) && !modifiers.contains(Modifiers::ALT)
        || modifiers.contains(Modifiers::META);

    match key {
        Key::Enter => PasswordEdit::Submit,
        Key::Backspace if is_shortcut => PasswordEdit::Clear,
        Key::Backspace => PasswordEdit::DeleteBackward,
        Key::Delete => PasswordEdit::DeleteForward,
        Key::ArrowLeft if is_shortcut => PasswordEdit::MoveCaret(CaretMove::Start),
        Key::ArrowRight if is_shortcut => PasswordEdit::MoveCaret(CaretMove::End),
        Key::ArrowLeft => PasswordEdit::MoveCaret(CaretMove::Left),
        Key::ArrowRight => PasswordEdit::MoveCaret(CaretMove::Right),
        Key::Home => PasswordEdit::MoveCaret(CaretMove::Start),
        Key::End => PasswordEdit::MoveCaret(CaretMove::End),
        Key::Character(text) if is_shortcut && text.eq_ignore_ascii_case("v") => {
            PasswordEdit::Paste
        }
        Key::Character(_) if is_shortcut => PasswordEdit::Ignore,
        Key::Character(text) => PasswordEdit::Insert(text.clone()),
        _ => PasswordEdit::Ignore,
    }
}

/// Returns the clipboard text without line breaks and other control characters.
fn clipboard_text() -> String {
    Application::try_global()
        .and_then(|application| application.clipboard().get_string())
        .map(|text| {
            text.chars()
                .filter(|character| !character.is_control())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(text: &str) -> Key {
        Key::Character(text.to_string())
    }

    fn edited(password: &str, caret: usize, edit: PasswordEdit) -> (String, usize) {
        let mut password = password.to_string();
        let caret = edit_password(&mut password, caret, &edit);
        (password, caret)
    }

    #[test]
    fn typed_characters_are_inserted() {
        assert_eq!(
            interpret_key(&character("a"), Modifiers::empty()),
            PasswordEdit::Insert("a".into())
        );
        assert_eq!(
            interpret_key(&character("A"), Modifiers::SHIFT),
            PasswordEdit::Insert("A".into())
        );
    }

    #[test]
    fn alt_gr_characters_are_inserted() {
        assert_eq!(
            interpret_key(&character("@"), Modifiers::CONTROL | Modifiers::ALT),
            PasswordEdit::Insert("@".into())
        );
    }

    #[test]
    fn control_shortcuts_paste_but_never_type() {
        assert_eq!(
            interpret_key(&character("v"), Modifiers::CONTROL),
            PasswordEdit::Paste
        );
        assert_eq!(
            interpret_key(&character("V"), Modifiers::META),
            PasswordEdit::Paste
        );
        assert_eq!(
            interpret_key(&character("a"), Modifiers::CONTROL),
            PasswordEdit::Ignore
        );
    }

    #[test]
    fn backspace_and_delete_remove_around_the_caret_and_control_backspace_clears() {
        assert_eq!(
            interpret_key(&Key::Backspace, Modifiers::empty()),
            PasswordEdit::DeleteBackward
        );
        assert_eq!(
            interpret_key(&Key::Delete, Modifiers::empty()),
            PasswordEdit::DeleteForward
        );
        assert_eq!(
            interpret_key(&Key::Backspace, Modifiers::CONTROL),
            PasswordEdit::Clear
        );
    }

    #[test]
    fn arrows_home_and_end_move_the_caret() {
        assert_eq!(
            interpret_key(&Key::ArrowLeft, Modifiers::empty()),
            PasswordEdit::MoveCaret(CaretMove::Left)
        );
        assert_eq!(
            interpret_key(&Key::ArrowRight, Modifiers::SHIFT),
            PasswordEdit::MoveCaret(CaretMove::Right)
        );
        assert_eq!(
            interpret_key(&Key::Home, Modifiers::empty()),
            PasswordEdit::MoveCaret(CaretMove::Start)
        );
        assert_eq!(
            interpret_key(&Key::End, Modifiers::empty()),
            PasswordEdit::MoveCaret(CaretMove::End)
        );
    }

    #[test]
    fn control_arrows_jump_to_the_ends() {
        assert_eq!(
            interpret_key(&Key::ArrowLeft, Modifiers::CONTROL),
            PasswordEdit::MoveCaret(CaretMove::Start)
        );
        assert_eq!(
            interpret_key(&Key::ArrowRight, Modifiers::CONTROL),
            PasswordEdit::MoveCaret(CaretMove::End)
        );
    }

    #[test]
    fn enter_submits_and_other_keys_are_ignored() {
        assert_eq!(
            interpret_key(&Key::Enter, Modifiers::empty()),
            PasswordEdit::Submit
        );
        assert_eq!(
            interpret_key(&Key::ArrowUp, Modifiers::empty()),
            PasswordEdit::Ignore
        );
    }

    #[test]
    fn text_is_inserted_at_the_caret() {
        assert_eq!(
            edited("acd", 1, PasswordEdit::Insert("b".into())),
            ("abcd".to_string(), 2)
        );
        assert_eq!(
            edited("ab", 2, PasswordEdit::Insert("cd".into())),
            ("abcd".to_string(), 4)
        );
    }

    #[test]
    fn backspace_removes_the_character_before_the_caret() {
        assert_eq!(
            edited("abc", 2, PasswordEdit::DeleteBackward),
            ("ac".to_string(), 1)
        );
        assert_eq!(
            edited("abc", 0, PasswordEdit::DeleteBackward),
            ("abc".to_string(), 0)
        );
    }

    #[test]
    fn delete_removes_the_character_after_the_caret() {
        assert_eq!(
            edited("abc", 1, PasswordEdit::DeleteForward),
            ("ac".to_string(), 1)
        );
        assert_eq!(
            edited("abc", 3, PasswordEdit::DeleteForward),
            ("abc".to_string(), 3)
        );
    }

    #[test]
    fn the_caret_stays_inside_the_password() {
        assert_eq!(
            edited("ab", 0, PasswordEdit::MoveCaret(CaretMove::Left)).1,
            0
        );
        assert_eq!(
            edited("ab", 2, PasswordEdit::MoveCaret(CaretMove::Right)).1,
            2
        );
        assert_eq!(
            edited("ab", 1, PasswordEdit::MoveCaret(CaretMove::Start)).1,
            0
        );
        assert_eq!(
            edited("ab", 0, PasswordEdit::MoveCaret(CaretMove::End)).1,
            2
        );
        assert_eq!(edited("ab", 9, PasswordEdit::Ignore).1, 2);
    }

    #[test]
    fn multibyte_characters_are_edited_whole() {
        assert_eq!(
            edited("ğüş", 2, PasswordEdit::DeleteBackward),
            ("ğş".to_string(), 1)
        );
        assert_eq!(
            edited("ğş", 1, PasswordEdit::Insert("ı".into())),
            ("ğış".to_string(), 2)
        );
    }

    #[test]
    fn clearing_empties_the_password_and_moves_the_caret_to_the_start() {
        assert_eq!(edited("secret", 3, PasswordEdit::Clear), (String::new(), 0));
    }

    #[test]
    fn the_mask_has_one_bullet_per_character() {
        assert_eq!(mask_for("ğa"), "••");
        assert_eq!(byte_offset("ğa", 1), 2);
        assert_eq!(byte_offset("ğa", 5), 3);
    }

    #[test]
    fn the_scroll_follows_the_caret_only_when_it_leaves_the_visible_area() {
        assert_eq!(scroll_offset_keeping_caret_visible(0.0, 50.0, 100.0), 0.0);
        assert_eq!(scroll_offset_keeping_caret_visible(0.0, 150.0, 100.0), 50.0);
        assert_eq!(scroll_offset_keeping_caret_visible(80.0, 30.0, 100.0), 30.0);
        assert_eq!(scroll_offset_keeping_caret_visible(20.0, 60.0, 100.0), 20.0);
        assert_eq!(scroll_offset_keeping_caret_visible(5.0, 10.0, -4.0), 10.0);
    }
}
