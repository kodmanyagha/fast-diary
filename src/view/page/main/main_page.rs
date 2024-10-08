use druid::{
    widget::{
        Button, CrossAxisAlignment, Flex, FlexParams, Label, LabelText, MainAxisAlignment, TextBox,
    },
    Insets, LocalizedString, Widget, WidgetExt,
};

use crate::modal::app_state::{AppPages, AppState};

pub fn build_ui() -> impl Widget<AppState> {
    let label_1 = Label::new(LocalizedString::new("page-login-title"))
        .padding(5.0)
        .center();
    let label_2 =
        Label::dynamic(|data: &AppState, _| format!("Current Page: {}", data.page.to_string()));

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
                        .lens(AppState::password),
                    FlexParams::new(70.0, CrossAxisAlignment::Center),
                )
                .with_flex_child(
                    Button::new("Start")
                        .on_click(|_, data: &mut AppState, _| data.page = AppPages::Diary)
                        .padding(5.0)
                        .expand_width(),
                    FlexParams::new(30.0, CrossAxisAlignment::Center),
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
