use druid::{
    theme,
    widget::{Flex, Label, Painter, ViewSwitcher},
    Env, PaintCtx, Rect, RenderContext, Size, Widget, WidgetExt,
};
use druid_material_icons::normal::action::{CALENDAR_MONTH, VIEW_LIST};

use crate::{
    consts::druid_selector,
    modal::{
        app_state::AppState,
        state::{diary_view_mode::DiaryViewMode, editor_mode::EditorMode},
    },
    utils::localization::translate,
    view::widget::icon_button::{paint_button_chrome, ICON_BUTTON_SIZE, ICON_COLOR},
};

const VIEW_MODE_BUTTON_WIDTH: f64 = 84.0;
const VIEW_MODE_ICON_SIZE: f64 = 16.0;
const VIEW_MODE_ICON_SPACING: f64 = 4.0;
const VIEW_MODE_TEXT_SIZE: f64 = 13.0;
const EDITOR_MODE_ICON_SIZE: f64 = 14.0;
const EDITOR_MODE_ICON_LINE_WIDTH: f64 = 1.5;

/// Builds a button of a fixed width that shows the icon and the name of the current diary view
/// mode. Clicking it switches to the other mode and stores the choice in the settings.
pub fn diary_view_mode_button() -> impl Widget<AppState> {
    let icon = ViewSwitcher::new(
        |app_state: &AppState, _env| app_state.diary_view_mode,
        |mode, _app_state, _env| {
            let paths = match mode {
                DiaryViewMode::List => VIEW_LIST,
                DiaryViewMode::Calendar => CALENDAR_MONTH,
            };

            Box::new(
                paths
                    .new(ICON_COLOR)
                    .fix_size(VIEW_MODE_ICON_SIZE, VIEW_MODE_ICON_SIZE),
            ) as Box<dyn Widget<AppState>>
        },
    );
    let name = Label::dynamic(|app_state: &AppState, _env| {
        translate(app_state.language, app_state.diary_view_mode.label_key())
    })
    .with_text_size(VIEW_MODE_TEXT_SIZE);

    Flex::row()
        .with_child(icon)
        .with_spacer(VIEW_MODE_ICON_SPACING)
        .with_child(name)
        .center()
        .fix_size(VIEW_MODE_BUTTON_WIDTH, ICON_BUTTON_SIZE)
        .background(Painter::new(paint_button_chrome))
        .on_click(|ctx, app_state: &mut AppState, _env| {
            app_state.diary_view_mode = app_state.diary_view_mode.toggled();
            ctx.submit_command(druid_selector::SETTINGS_SAVE);
        })
}

/// Builds a small button that shows the current editor mode as a square: empty for editing, half
/// filled for the split view and filled for the preview. Clicking it switches to the next mode
/// and stores the choice in the settings.
pub fn editor_mode_button() -> impl Widget<EditorMode> {
    Painter::new(paint_editor_mode_button)
        .fix_size(ICON_BUTTON_SIZE, ICON_BUTTON_SIZE)
        .on_click(|ctx, mode: &mut EditorMode, _env| {
            *mode = mode.next();
            ctx.submit_command(druid_selector::SETTINGS_SAVE);
        })
}

/// Returns the part of the `icon` square that is filled for `mode`.
fn filled_part(icon: Rect, mode: EditorMode) -> Option<Rect> {
    match mode {
        EditorMode::Edit => None,
        EditorMode::Split => Some(Rect::new(icon.center().x, icon.y0, icon.x1, icon.y1)),
        EditorMode::Preview => Some(icon),
    }
}

fn paint_editor_mode_button(ctx: &mut PaintCtx, mode: &EditorMode, env: &Env) {
    let icon_color = env.get(theme::TEXT_COLOR);
    let icon = Rect::from_center_size(
        ctx.size().to_rect().center(),
        Size::new(EDITOR_MODE_ICON_SIZE, EDITOR_MODE_ICON_SIZE),
    );

    paint_button_chrome(ctx, mode, env);
    if let Some(filled) = filled_part(icon, *mode) {
        ctx.fill(filled, &icon_color);
    }
    ctx.stroke(icon, &icon_color, EDITOR_MODE_ICON_LINE_WIDTH);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICON: Rect = Rect::new(10.0, 20.0, 24.0, 34.0);

    #[test]
    fn the_edit_icon_is_empty() {
        assert_eq!(filled_part(ICON, EditorMode::Edit), None);
    }

    #[test]
    fn the_split_icon_is_filled_on_its_right_half() {
        assert_eq!(
            filled_part(ICON, EditorMode::Split),
            Some(Rect::new(17.0, 20.0, 24.0, 34.0))
        );
    }

    #[test]
    fn the_preview_icon_is_completely_filled() {
        assert_eq!(filled_part(ICON, EditorMode::Preview), Some(ICON));
    }
}
