use druid::{
    widget::{
        Button, CrossAxisAlignment, Flex, FlexParams, Label, LabelText, MainAxisAlignment,
        RadioGroup, TextBox,
    },
    Color, Insets, LocalizedString, UnitPoint, Widget, WidgetExt,
};
use druid_widget_nursery::{dropdown::DROPDOWN_SHOW, Dropdown};

use crate::modal::app_data::{AppData, AppPages};

pub fn build_ui() -> impl Widget<AppData> {
    let label_1 = Label::new(LocalizedString::new("page-login-title"))
        .padding(5.0)
        .center();
    let label_2 =
        Label::dynamic(|data: &AppData, _| format!("Current Page: {}", data.page.to_string()));

    Flex::column()
        .with_child(label_1)
        .with_default_spacer()
        .with_child(label_2)
        .with_default_spacer()
        .with_child(
            Flex::row()
                .must_fill_main_axis(true)
                .with_flex_child(
                    TextBox::new()
                        .with_placeholder(LabelText::Localized(LocalizedString::new(
                            "page-login-enterPassword",
                        )))
                        .expand_width()
                        .lens(AppData::password),
                    FlexParams::new(12.0, CrossAxisAlignment::Center),
                )
                .with_flex_child(
                    Button::new("Start")
                        .on_click(|_, data: &mut AppData, _| data.page = AppPages::Diary)
                        .padding(5.0)
                        .expand_width(),
                    FlexParams::new(2.0, CrossAxisAlignment::Center),
                )
                .fix_width(400f64),
        )
        .must_fill_main_axis(true)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::Center)
        .expand_height()
        .expand_width()
        .padding(Insets::uniform(10.0))
}
