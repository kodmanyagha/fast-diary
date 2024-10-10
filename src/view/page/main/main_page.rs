use druid::{
    widget::{
        Button, CrossAxisAlignment, FillStrat, Flex, FlexParams, Image, Label, LabelText,
        MainAxisAlignment, TextBox,
    },
    Color, FileDialogOptions, Insets, LocalizedString, Widget, WidgetExt,
};

use crate::{
    modal::app_state::{AppPages, AppState, OpenFilePurpose},
    utils::get_image::get_image,
};

pub fn build_ui() -> impl Widget<AppState> {
    let label_welcome = Label::new(LocalizedString::new("page-login-title"));
    let label_welcome_sub = Label::new(LocalizedString::new("page-login-welcomeSub1"));

    Flex::column()
        .with_flex_child(
            Image::new(get_image("./resources/images/diary_icon_1.png"))
                .fill_mode(FillStrat::Contain)
                .padding(10_f64)
                .fix_height(150_f64),
            FlexParams::new(30_f64, CrossAxisAlignment::Center),
        )
        .with_default_spacer()
        .with_default_spacer()
        .with_flex_child(
            Flex::column()
                .with_flex_child(
                    label_welcome,
                    FlexParams::new(50_f64, CrossAxisAlignment::Center),
                )
                .with_flex_child(
                    label_welcome_sub,
                    FlexParams::new(50_f64, CrossAxisAlignment::Center),
                ),
            FlexParams::new(30_f64, CrossAxisAlignment::Center),
        )
        .with_default_spacer()
        .with_default_spacer()
        .with_flex_child(
            Flex::column()
                .with_flex_child(
                    Label::new(LocalizedString::new("page-login-selectFolder")),
                    FlexParams::new(100_f64, CrossAxisAlignment::Start),
                )
                .with_flex_child(
                    Flex::row()
                        .with_flex_child(
                            TextBox::new().expand_width().lens(AppState::password),
                            FlexParams::new(70_f64, CrossAxisAlignment::Start),
                        )
                        .with_flex_child(
                            Button::new(LocalizedString::new("page-login-selectFolder"))
                                .on_click(|ctx, data: &mut AppState, _| {
                                    data.open_file_purpose = OpenFilePurpose::DiaryPath;

                                    let dialog_options = FileDialogOptions::new()
                                        .select_directories()
                                        .show_hidden()
                                        .name_label("Source")
                                        .title("Select a folder")
                                        .button_text("Select");

                                    ctx.submit_command(
                                        druid::commands::SHOW_OPEN_PANEL
                                            .with(dialog_options.clone()),
                                    )
                                })
                                .padding(1_f64)
                                .expand_width(),
                            FlexParams::new(30_f64, CrossAxisAlignment::Start),
                        ),
                    FlexParams::new(100_f64, CrossAxisAlignment::Start),
                )
                .with_default_spacer()
                .with_flex_child(
                    Label::new(LocalizedString::new("page-login-enterPassword")),
                    FlexParams::new(100_f64, CrossAxisAlignment::Start),
                )
                .with_flex_child(
                    TextBox::new().expand_width().lens(AppState::password),
                    FlexParams::new(100_f64, CrossAxisAlignment::Start),
                )
                .with_default_spacer()
                .with_flex_child(
                    Button::new("Start")
                        .on_click(|_, data: &mut AppState, _| data.page = AppPages::Diary)
                        .expand(),
                    FlexParams::new(100_f64, CrossAxisAlignment::Start),
                )
                .fix_width(400f64),
            FlexParams::new(60_f64, CrossAxisAlignment::Center),
        )
        .must_fill_main_axis(true)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_alignment(MainAxisAlignment::Start)
        .expand_height()
        .expand_width()
}
