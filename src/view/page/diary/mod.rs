use druid::{
    widget::{
        BackgroundBrush, Container, CrossAxisAlignment, Flex, FlexParams, Label, List, Scroll,
        Split, TextBox,
    },
    Color, Insets, LensExt, LinearGradient, LocalizedString, UnitPoint, Widget, WidgetExt,
};

use crate::modal::app_data::{AppData, DiaryListItem};

fn make_list_item() -> impl Widget<DiaryListItem> {
    Container::new(
        Flex::row()
            .with_flex_child(
                TextBox::new().lens(DiaryListItem::title).expand_width(),
                FlexParams::new(33.3, Some(druid::widget::CrossAxisAlignment::Center)),
            )
            .with_default_spacer()
            .with_flex_child(
                Label::dynamic(|d: &DiaryListItem, event| d.title.to_owned()),
                FlexParams::new(33.3, Some(druid::widget::CrossAxisAlignment::Center)),
            )
            .with_default_spacer()
            .with_flex_child(
                Label::dynamic(|d: &DiaryListItem, event| d.date.to_string()),
                FlexParams::new(33.3, Some(druid::widget::CrossAxisAlignment::Center)),
            )
            .padding((5.0, 10.0))
            .background(BackgroundBrush::Linear(LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (Color::rgb8(128, 128, 128), Color::rgb8(105, 105, 105)),
            )))
            .rounded(10.0),
    )
    .padding((0.0, 5.0))
}

pub fn build_ui() -> impl Widget<AppData> {
    let label_1 = Label::new(LocalizedString::new("page-diary-title"))
        .padding(5.0)
        .center();

    let label_2 =
        Label::dynamic(|data: &AppData, _| format!("Current page: {}", data.page.to_string()));

    Flex::column()
        .with_flex_child(
            Split::columns(
                Flex::column()
                    .with_flex_child(
                        label_1,
                        FlexParams::new(10.0, Some(druid::widget::CrossAxisAlignment::Start)),
                    )
                    .with_flex_child(
                        Scroll::new(List::new(make_list_item).lens(AppData::diaries))
                            .vertical()
                            .expand_width()
                            .expand_height(),
                        FlexParams::new(90.0, Some(druid::widget::CrossAxisAlignment::Start)),
                    )
                    .expand_width()
                    .expand_height()
                    .padding(Insets::uniform(10.0)),
                Flex::column()
                    .with_child(label_2)
                    .expand_width()
                    .expand_height()
                    .padding(Insets::uniform(10.0)),
            )
            .draggable(true)
            .bar_size(3f64)
            .solid_bar(true)
            .min_size(150f64, 400f64)
            .expand_width()
            .expand_height(),
            FlexParams::new(100.0, Some(druid::widget::CrossAxisAlignment::Center)),
        )
        .expand_height()
        .expand_width()
}
