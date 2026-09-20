use druid::{
    theme, widget::Painter, Color, Env, EventCtx, PaintCtx, RenderContext, Widget, WidgetExt,
};
use druid_material_icons::IconPaths;
use druid_widget_nursery::WidgetExt as NurseryWidgetExt;

use crate::{modal::app_state::AppState, utils::localization::text};

pub const ICON_BUTTON_SIZE: f64 = 28.0;

const ICON_SIZE: f64 = 18.0;
pub const ICON_COLOR: Color = Color::grey8(0xf0);
const BUTTON_CORNER_RADIUS: f64 = 4.0;

/// Paints the background of a small square button: it gets lighter while the mouse is over it and
/// darker while it is pressed.
pub fn paint_button_chrome<T>(ctx: &mut PaintCtx, _data: &T, env: &Env) {
    let button = ctx.size().to_rect().to_rounded_rect(BUTTON_CORNER_RADIUS);
    let background = match (ctx.is_active(), ctx.is_hot()) {
        (true, _) => env.get(theme::BUTTON_DARK),
        (false, true) => env.get(theme::BUTTON_LIGHT),
        (false, false) => env.get(theme::BACKGROUND_LIGHT),
    };

    ctx.fill(button, &background);
    ctx.stroke(button, &env.get(theme::BORDER_DARK), 1.0);
}

/// Builds a small square button that only shows `icon`. The text under `tooltip_key` in the
/// translations is shown when the mouse rests on the button.
pub fn icon_button(
    icon: IconPaths,
    tooltip_key: &'static str,
    on_click: impl Fn(&mut EventCtx, &mut AppState, &Env) + 'static,
) -> impl Widget<AppState> {
    icon.new(ICON_COLOR)
        .fix_size(ICON_SIZE, ICON_SIZE)
        .center()
        .fix_size(ICON_BUTTON_SIZE, ICON_BUTTON_SIZE)
        .background(Painter::new(paint_button_chrome))
        .on_click(on_click)
        .tooltip(text(tooltip_key))
}
