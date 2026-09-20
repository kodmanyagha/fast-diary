use druid::{
    widget::{
        Button, CrossAxisAlignment, Either, FillStrat, Flex, Image, Label, MainAxisAlignment,
        Scroll, SizedBox, ViewSwitcher,
    },
    Color, FileDialogOptions, LocalizedString, Widget, WidgetExt,
};
use druid_widget_nursery::ListSelect;

use crate::{
    consts::druid_selector,
    modal::app_state::{AppState, DiaryBasePathLens, FolderProtection, OpenFilePurpose},
    utils::get_image::get_image,
    vault::password::weak_password_warning,
    view::widget::password_box::password_box,
};

const RECENT_FOLDERS_LIST_HEIGHT: f64 = 120_f64;
const WARNING_COLOR: Color = Color::rgb8(200, 130, 0);
const ERROR_COLOR: Color = Color::rgb8(200, 60, 60);

fn is_plain_folder_selected(data: &AppState) -> bool {
    data.diary_base_path.is_some() && data.folder_protection == FolderProtection::Plain
}

/// Builds the part of the page that asks for the password of the selected folder and, for
/// a folder that is not encrypted yet, offers to encrypt it.
fn build_password_section() -> impl Widget<AppState> {
    let folder_status = Label::dynamic(|data: &AppState, _env| {
        match (&data.diary_base_path, data.folder_protection) {
            (None, _) => "",
            (Some(_), FolderProtection::Plain) => "This folder is not encrypted.",
            (Some(_), FolderProtection::Encrypted) => "This folder is encrypted.",
        }
        .to_string()
    });

    let encrypt_section = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new(
            "Confirm password (only needed to encrypt this folder)",
        ))
        .with_default_spacer()
        .with_child(
            password_box(druid_selector::FOLDER_ENCRYPT).lens(AppState::password_confirmation),
        )
        .with_default_spacer()
        .with_child(
            Label::dynamic(|data: &AppState, _env| {
                weak_password_warning(&data.password)
                    .unwrap_or_default()
                    .to_string()
            })
            .with_text_color(WARNING_COLOR),
        )
        .with_default_spacer()
        .with_child(
            Button::new("Encrypt this folder")
                .on_click(|ctx, _data: &mut AppState, _| {
                    ctx.submit_command(druid_selector::FOLDER_ENCRYPT)
                })
                .expand_width(),
        )
        .with_default_spacer();

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(folder_status)
        .with_default_spacer()
        .with_child(Label::new(LocalizedString::new("page-login-enterPassword")))
        .with_default_spacer()
        .with_child(password_box(druid_selector::FOLDER_OPEN).lens(AppState::password))
        .with_default_spacer()
        .with_child(Either::new(
            |data: &AppState, _env| is_plain_folder_selected(data),
            encrypt_section,
            SizedBox::empty(),
        ))
        .with_child(
            Label::dynamic(|data: &AppState, _env| data.status_message.clone())
                .with_text_color(ERROR_COLOR),
        )
        .with_default_spacer()
}

fn build_recent_folders_list() -> impl Widget<AppState> {
    let list_select = ViewSwitcher::new(
        |data: &AppState, _env| data.recent_folders.clone(),
        |recent_folders, _data, _env| {
            let items = recent_folders
                .iter()
                .map(|path| (path.clone(), path.clone()));

            Box::new(
                ListSelect::new(items)
                    .lens(DiaryBasePathLens)
                    .expand_width(),
            ) as Box<dyn Widget<AppState>>
        },
    );

    Scroll::new(list_select)
        .vertical()
        .fix_height(RECENT_FOLDERS_LIST_HEIGHT)
        .expand_width()
}

pub fn build_ui() -> impl Widget<AppState> {
    let label_welcome = Label::new(LocalizedString::new("page-login-title"));
    let label_welcome_sub = Label::new(LocalizedString::new("page-login-welcomeSub1"));

    Flex::column()
        .with_child(
            Image::new(get_image("./resources/images/diary_icon_1.png"))
                .fill_mode(FillStrat::Contain)
                .padding(10_f64)
                .fix_height(100_f64),
        )
        .with_default_spacer()
        .with_child(
            Flex::column()
                .with_child(label_welcome)
                .with_default_spacer()
                .with_child(label_welcome_sub)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .with_default_spacer()
        .with_child(
            Flex::column()
                .with_child(Label::new(LocalizedString::new("page-login-selectFolder")))
                .with_default_spacer()
                .with_child(
                    Label::dynamic(|data: &AppState, _env| {
                        if let Some(selected_path) = data.diary_base_path.clone() {
                            selected_path
                        } else {
                            "".to_string()
                        }
                    })
                    .expand_width(),
                )
                .with_default_spacer()
                .with_child(
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
                                druid::commands::SHOW_OPEN_PANEL.with(dialog_options.clone()),
                            )
                        })
                        .expand_width(),
                )
                .with_default_spacer()
                .with_child(Label::new(LocalizedString::new("page-login-recentFolders")))
                .with_default_spacer()
                .with_child(build_recent_folders_list())
                .with_default_spacer()
                .with_child(build_password_section())
                .with_child(
                    Button::new(LocalizedString::new("page-login-start"))
                        .on_click(|ctx, _data: &mut AppState, _| {
                            ctx.submit_command(druid_selector::FOLDER_OPEN)
                        })
                        .disabled_if(|data, _| {
                            data.diary_base_path.is_none()
                                || (data.folder_protection == FolderProtection::Encrypted
                                    && data.password.is_empty())
                        })
                        .expand_width(),
                )
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .fix_width(400_f64),
        )
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_alignment(MainAxisAlignment::Center)
        .expand_height()
        .expand_width()
}
