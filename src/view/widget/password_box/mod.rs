use druid::{
    keyboard_types::Key,
    widget::{Controller, Label, Painter},
    Application, Color, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, Modifiers, PaintCtx,
    RenderContext, Selector, Widget, WidgetExt,
};

const MASK_CHARACTER: &str = "•";
const BORDER_COLOR: Color = Color::grey8(128);
const FOCUS_COLOR: Color = Color::rgb8(70, 130, 180);

#[derive(Debug, Clone, PartialEq, Eq)]
enum PasswordEdit {
    Append(String),
    DeleteLast,
    Clear,
    Paste,
    Submit,
    Ignore,
}

/// Builds a single line input that shows one bullet per typed character instead of the text.
/// Pressing Enter submits `on_enter`.
pub fn password_box(on_enter: Selector<()>) -> impl Widget<String> {
    Label::dynamic(|password: &String, _env| MASK_CHARACTER.repeat(password.chars().count()))
        .align_left()
        .padding((8.0, 6.0))
        .expand_width()
        .controller(PasswordInputController { on_enter })
        .background(Painter::new(paint_border))
}

/// Draws the border of the input, highlighted while the input has the keyboard focus.
fn paint_border(ctx: &mut PaintCtx, _password: &String, _env: &Env) {
    let (color, width) = if ctx.has_focus() {
        (FOCUS_COLOR, 2.0)
    } else {
        (BORDER_COLOR, 1.0)
    };
    let bounds = ctx.size().to_rect().inset(-width / 2.0);

    ctx.stroke(bounds, &color, width);
}

struct PasswordInputController {
    on_enter: Selector<()>,
}

impl PasswordInputController {
    /// Applies `edit` to the typed password.
    fn apply(&self, ctx: &mut EventCtx, password: &mut String, edit: PasswordEdit) {
        match edit {
            PasswordEdit::Append(text) => password.push_str(&text),
            PasswordEdit::DeleteLast => {
                password.pop();
            }
            PasswordEdit::Clear => password.clear(),
            PasswordEdit::Paste => password.push_str(&clipboard_text()),
            PasswordEdit::Submit => ctx.submit_command(self.on_enter),
            PasswordEdit::Ignore => return,
        }
        ctx.set_handled();
    }
}

impl<W: Widget<String>> Controller<String, W> for PasswordInputController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        password: &mut String,
        env: &Env,
    ) {
        match event {
            Event::MouseDown(_) => ctx.request_focus(),
            Event::KeyDown(key_event) if ctx.is_focused() => {
                self.apply(ctx, password, interpret_key(&key_event.key, key_event.mods));
            }
            _ => {}
        }

        child.event(ctx, event, password, env)
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        password: &String,
        env: &Env,
    ) {
        match event {
            LifeCycle::BuildFocusChain => ctx.register_for_focus(),
            LifeCycle::FocusChanged(_) => ctx.request_paint(),
            _ => {}
        }

        child.lifecycle(ctx, event, password, env)
    }
}

/// Translates a key press into an edit of the password. Ctrl+Alt is treated as AltGr, so
/// characters typed with AltGr are appended like any other.
fn interpret_key(key: &Key, modifiers: Modifiers) -> PasswordEdit {
    let is_shortcut = modifiers.contains(Modifiers::CONTROL) && !modifiers.contains(Modifiers::ALT)
        || modifiers.contains(Modifiers::META);

    match key {
        Key::Enter => PasswordEdit::Submit,
        Key::Backspace if is_shortcut => PasswordEdit::Clear,
        Key::Backspace => PasswordEdit::DeleteLast,
        Key::Character(text) if is_shortcut && text.eq_ignore_ascii_case("v") => {
            PasswordEdit::Paste
        }
        Key::Character(_) if is_shortcut => PasswordEdit::Ignore,
        Key::Character(text) => PasswordEdit::Append(text.clone()),
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

    #[test]
    fn typed_characters_are_appended() {
        assert_eq!(
            interpret_key(&character("a"), Modifiers::empty()),
            PasswordEdit::Append("a".into())
        );
        assert_eq!(
            interpret_key(&character("A"), Modifiers::SHIFT),
            PasswordEdit::Append("A".into())
        );
    }

    #[test]
    fn alt_gr_characters_are_appended() {
        assert_eq!(
            interpret_key(&character("@"), Modifiers::CONTROL | Modifiers::ALT),
            PasswordEdit::Append("@".into())
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
    fn backspace_deletes_and_control_backspace_clears() {
        assert_eq!(
            interpret_key(&Key::Backspace, Modifiers::empty()),
            PasswordEdit::DeleteLast
        );
        assert_eq!(
            interpret_key(&Key::Backspace, Modifiers::CONTROL),
            PasswordEdit::Clear
        );
    }

    #[test]
    fn enter_submits_and_other_keys_are_ignored() {
        assert_eq!(
            interpret_key(&Key::Enter, Modifiers::empty()),
            PasswordEdit::Submit
        );
        assert_eq!(
            interpret_key(&Key::ArrowLeft, Modifiers::empty()),
            PasswordEdit::Ignore
        );
    }
}
