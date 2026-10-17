use druid::{
    widget::{Button, CrossAxisAlignment, Either, Flex, Label},
    Color, Insets, LensExt, Widget, WidgetExt,
};

use crate::{
    consts::druid_selector,
    modal::{
        app_state::{AppState, FolderProtection, PasswordChangeForm},
        state::app_pages::AppPages,
    },
    utils::localization::{password_weakness_text, text},
    view::widget::password_box::password_box,
};

const WARNING_COLOR: Color = Color::rgb8(200, 130, 0);
const STATUS_COLOR: Color = Color::rgb8(200, 60, 60);
const FORM_WIDTH: f64 = 400_f64;

/// Builds the form that changes the password of the opened encrypted folder.
fn build_change_password_form() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new(text("page-settings-changeFolderPassword")))
        .with_default_spacer()
        .with_child(Label::new(text("page-settings-currentPassword")))
        .with_child(
            password_box(druid_selector::FOLDER_CHANGE_PASSWORD)
                .lens(AppState::password_change.then(PasswordChangeForm::current_password)),
        )
        .with_default_spacer()
        .with_child(Label::new(text("page-settings-newPassword")))
        .with_child(
            password_box(druid_selector::FOLDER_CHANGE_PASSWORD)
                .lens(AppState::password_change.then(PasswordChangeForm::new_password)),
        )
        .with_child(
            Label::dynamic(|data: &AppState, _env| {
                password_weakness_text(data.language, &data.password_change.new_password)
            })
            .with_text_color(WARNING_COLOR),
        )
        .with_default_spacer()
        .with_child(Label::new(text("page-settings-confirmNewPassword")))
        .with_child(
            password_box(druid_selector::FOLDER_CHANGE_PASSWORD)
                .lens(AppState::password_change.then(PasswordChangeForm::confirmation)),
        )
        .with_default_spacer()
        .with_child(
            Button::new(text("page-settings-changePassword"))
                .on_click(|ctx, _data: &mut AppState, _| {
                    ctx.submit_command(druid_selector::FOLDER_CHANGE_PASSWORD)
                })
                .expand_width(),
        )
        .with_default_spacer()
        .with_child(
            Label::dynamic(|data: &AppState, _env| data.status_message.clone())
                .with_text_color(STATUS_COLOR),
        )
        .fix_width(FORM_WIDTH)
}

pub fn build_ui() -> impl Widget<AppState> {
    let label_1 = Label::new(text("page-settings-title"))
        .padding(5.0)
        .center();

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(label_1)
        .with_default_spacer()
        .with_child(Either::new(
            |data: &AppState, _env| data.folder_protection == FolderProtection::Encrypted,
            build_change_password_form(),
            Label::new(text("page-settings-folderNotEncrypted")),
        ))
        .with_default_spacer()
        .with_child(Button::new(text("page-settings-back")).on_click(
            |_ctx, data: &mut AppState, _| {
                data.status_message.clear();
                data.page = AppPages::Diary;
            },
        ))
        .expand_height()
        .expand_width()
        .padding(Insets::uniform(10.0))
}
