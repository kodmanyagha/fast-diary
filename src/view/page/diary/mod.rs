use druid::{
    widget::{Flex, FlexParams, Label, List, Scroll, Split, TextBox},
    Color, Insets, LensExt, LocalizedString, Widget, WidgetExt,
};

use crate::modal::app_data::{AppData, DiaryListItem};

fn make_list_item() -> impl Widget<DiaryListItem> {
    Flex::row()
        .with_child(TextBox::new().lens(DiaryListItem::title))
        .with_child(Label::dynamic(|d: &DiaryListItem, event| d.title.to_owned()).padding(5.0))
        .with_child(Label::dynamic(|d: &DiaryListItem, event| d.date.to_string()).padding(5.0))
        .expand_width()
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
                    .with_child(label_1)
                    .with_child(
                        Scroll::new(List::new(make_list_item).lens(AppData::diaries)).vertical(),
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
            .expand_height()
            .border(Color::rgb8(255, 69, 0), 1f64),
            FlexParams::new(100.0, Some(druid::widget::CrossAxisAlignment::Center)),
        )
        .expand_height()
        .expand_width()
        .border(Color::rgb8(240, 230, 140), 0f64)
}
