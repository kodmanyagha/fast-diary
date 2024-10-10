use druid::{
    widget::{BackgroundBrush, Container, CrossAxisAlignment, Flex, FlexParams, Label, TextBox},
    Color, Command, LinearGradient, Target, UnitPoint, Widget, WidgetExt,
};

use crate::{
    modal::app_state::DiaryListItem, view::window::main::main_window_controller::DIARY_SET_CURRENT,
};

pub fn create_diary_list_item() -> impl Widget<DiaryListItem> {
    Container::new(
        Flex::row()
            .with_flex_child(
                Label::dynamic(|d: &DiaryListItem, event| d.title.to_owned()).expand_width(),
                FlexParams::new(67.0, Some(CrossAxisAlignment::Center)),
            )
            .with_default_spacer()
            .with_flex_child(
                Label::dynamic(|d: &DiaryListItem, event| d.date.to_string()[0..10].to_string()),
                FlexParams::new(33.0, Some(CrossAxisAlignment::Center)),
            )
            .padding((5.0, 10.0))
            .background(BackgroundBrush::Linear(LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (Color::rgb8(128, 128, 128), Color::rgb8(105, 105, 105)),
            )))
            .rounded(10.0)
            .on_click(|ctx, data, _env| {
                ctx.submit_command(Command::new(
                    DIARY_SET_CURRENT,
                    data.to_owned(),
                    Target::Auto,
                ));
            }),
    )
    .padding((0.0, 5.0))
}
