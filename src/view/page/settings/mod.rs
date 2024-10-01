use druid::{
    widget::{Flex, Label},
    Color, Insets, LocalizedString, Widget, WidgetExt,
};

use crate::modal::app_data::AppData;

pub fn build_ui() -> impl Widget<AppData> {
    let label_1 = Label::new(LocalizedString::new("page-settings-title"))
        .padding(5.0)
        .center();
    let label_2 = Label::dynamic(|data: &AppData, _| format!("{}", data.page.to_string()));

    Flex::column()
        .with_child(label_1)
        .with_child(label_2)
        .expand_height()
        .expand_width()
        .padding(Insets::uniform(10.0))
        .border(Color::rgb8(240, 230, 140), 1f64)
}
