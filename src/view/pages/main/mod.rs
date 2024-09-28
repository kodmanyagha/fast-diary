use druid::{
    widget::{Button, Flex, Label, RadioGroup, TextBox},
    Insets, LocalizedString, UnitPoint, Widget, WidgetExt,
};
use druid_widget_nursery::{dropdown::DROPDOWN_SHOW, Dropdown};

use crate::modal::app_data::{AppData, AppPages};

pub fn build_ui() -> impl Widget<AppData> {
    let label_1 = Label::new(LocalizedString::new("page-login-title"))
        .padding(5.0)
        .center();
    let label_2 = Label::dynamic(|data: &AppData, _| format!("{}", data.page.to_string()));
    // let label_2 = Label::new("Label 2".to_string());

    let textbox1 = TextBox::new()
        .with_placeholder("Test")
        .lens(AppData::app_title);

    let button_1 = Button::new("Click to me")
        .on_click(|_ctx, data: &mut AppData, _env| data.page = AppPages::Diary)
        .padding(5.0);

    let dropdown_1 = Dropdown::new(
        Flex::row()
            .with_child(TextBox::new())
            .with_default_spacer()
            .with_child(Button::new("V").on_click(|ctx, _data, _env| {
                ctx.submit_notification(DROPDOWN_SHOW);
            })),
        |_data, _env| {
            let places: Vec<(&'static str, String)> = vec!["England", "San Tropez", "Antarctica"]
                .into_iter()
                .map(|item| (item, item.to_owned()))
                .collect();
            RadioGroup::column(places)
        },
    )
    .align_vertical(UnitPoint::CENTER)
    .align_horizontal(UnitPoint::CENTER)
    .lens(AppData::app_title);

    let flex = Flex::column()
        .with_child(label_1)
        .with_default_spacer()
        .with_child(label_2)
        .with_default_spacer()
        .with_child(button_1)
        .with_default_spacer()
        .with_child(textbox1)
        .with_default_spacer()
        .with_child(dropdown_1)
        .padding(Insets::uniform(10.0));

    flex
}
