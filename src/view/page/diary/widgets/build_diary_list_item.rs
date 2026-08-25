use druid::{
    widget::{Container, CrossAxisAlignment, Flex, FlexParams, Label, Painter},
    Color, LinearGradient, RenderContext, Target, UnitPoint, Widget, WidgetExt,
};

use crate::{
    consts::druid_selector, modal::state::diary_list_item::DiaryListItem,
    utils::event_sink::get_event_sink,
};

const NORMAL_GRADIENT: (Color, Color) = (Color::rgb8(128, 128, 128), Color::rgb8(105, 105, 105));
const SELECTED_GRADIENT: (Color, Color) = (Color::rgb8(70, 130, 180), Color::rgb8(51, 92, 130));

pub fn build_diary_list_item() -> impl Widget<(DiaryListItem, bool)> {
    let background = Painter::new(|ctx, (_, is_selected): &(DiaryListItem, bool), _env| {
        let bounds = ctx.size().to_rect();
        let gradient = if *is_selected {
            SELECTED_GRADIENT
        } else {
            NORMAL_GRADIENT
        };

        ctx.fill(
            bounds,
            &LinearGradient::new(UnitPoint::TOP, UnitPoint::BOTTOM, gradient),
        );
    });

    Container::new(
        Flex::row()
            .with_flex_child(
                Label::dynamic(|d: &DiaryListItem, _env| d.date.to_string()[0..19].to_string()),
                FlexParams::new(67.0, Some(CrossAxisAlignment::Center)),
            )
            .with_default_spacer()
            .with_flex_child(
                Label::dynamic(|d: &DiaryListItem, _env| d.summary.to_owned()).expand_width(),
                FlexParams::new(33.0, Some(CrossAxisAlignment::Center)),
            )
            .lens(druid::lens!((DiaryListItem, bool), 0))
            .padding((5.0, 10.0))
            .background(background)
            .rounded(10.0)
            .on_click(|_ctx, (data, _is_selected), _env| {
                let data = data.to_owned();
                let Some(event_sink) = get_event_sink() else {
                    return;
                };

                tokio::spawn(async move {
                    let _ = event_sink.submit_command(
                        druid_selector::DIARY_SAVE_CURRENT,
                        (),
                        Target::Global,
                    );

                    let _ = event_sink.submit_command(
                        druid_selector::DIARY_SET_CURRENT,
                        data,
                        Target::Global,
                    );
                });
            }),
    )
    .padding((0.0, 5.0))
}
