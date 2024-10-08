use druid::{
    widget::{Flex, Label},
    Insets, LocalizedString, Widget, WidgetExt,
};

use crate::modal::app_state::AppState;

pub fn build_ui() -> impl Widget<AppState> {
    let label_1 = Label::new(LocalizedString::new("page-settings-title"))
        .padding(5.0)
        .center();
    let label_2 = Label::dynamic(|data: &AppState, _| format!("{}", data.page.to_string()));

    Flex::column()
        .with_child(label_1)
        .with_child(label_2)
        .expand_height()
        .expand_width()
        .padding(Insets::uniform(10.0))
}
