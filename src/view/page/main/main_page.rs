use druid::{
    widget::{
        Button, CrossAxisAlignment, Either, FillStrat, Flex, Image, Label, LineBreaking, Scroll,
        SizedBox, Split, ViewSwitcher,
    },
    Color, FileDialogOptions, Widget, WidgetExt,
};
use druid_widget_nursery::ListSelect;

use crate::{
    config::window_settings::MIN_WINDOW_SIZE,
    consts::druid_selector,
    modal::app_state::{AppState, DiaryBasePathLens, FolderProtection, OpenFilePurpose},
    utils::{display_text::shorten_in_the_middle, get_image::diary_icon, localization::text},
    vault::password::weak_password_warning,
    view::widget::password_box::password_box,
};

const PAGE_CONTENT_WIDTH: f64 = MIN_WINDOW_SIZE.width;
const COLUMN_PADDING: f64 = 24.0;
const COLUMN_MIN_WIDTH: f64 = 320.0;
const DIVIDER_WIDTH: f64 = 2.0;
const OPEN_BUTTON_HEIGHT: f64 = 64.0;
const MAX_FOLDER_PATH_CHARS: usize = 40;
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
        .with_child(
            Label::new("Confirm password (only needed to encrypt this folder)")
                .with_line_break_mode(LineBreaking::WordWrap),
        )
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
            .with_text_color(WARNING_COLOR)
            .with_line_break_mode(LineBreaking::WordWrap),
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
        .with_child(Label::new(text("page-login-enterPassword")))
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
                .with_text_color(ERROR_COLOR)
                .with_line_break_mode(LineBreaking::WordWrap),
        )
        .with_default_spacer()
}

/// Shortens the path of a folder so that it fits in the column, keeping its beginning and its end.
fn shorten_folder_path(path: &str) -> String {
    shorten_in_the_middle(path, MAX_FOLDER_PATH_CHARS)
}

fn build_recent_folders_list() -> impl Widget<AppState> {
    let list_select = ViewSwitcher::new(
        |data: &AppState, _env| data.recent_folders.clone(),
        |recent_folders, _data, _env| {
            let items = recent_folders
                .iter()
                .map(|path| (shorten_folder_path(path), path.clone()));

            Box::new(
                ListSelect::new(items)
                    .lens(DiaryBasePathLens)
                    .expand_width(),
            ) as Box<dyn Widget<AppState>>
        },
    );

    Scroll::new(list_select).vertical().expand()
}

/// Returns the options of the dialog that asks for the folder of the diaries.
pub fn folder_dialog_options() -> FileDialogOptions {
    FileDialogOptions::new()
        .select_directories()
        .show_hidden()
        .name_label("Source")
        .title("Select a folder")
        .button_text("Select")
}

fn build_select_folder_button() -> impl Widget<AppState> {
    Button::new(text("page-login-selectFolder"))
        .on_click(|ctx, data: &mut AppState, _| {
            data.open_file_purpose = OpenFilePurpose::DiaryPath;
            ctx.submit_command(druid::commands::SHOW_OPEN_PANEL.with(folder_dialog_options()))
        })
        .expand_width()
}

/// Builds the left half of the page: the folder chooser and the history of chosen folders.
fn build_folder_column() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new(text("page-login-selectFolder")))
        .with_default_spacer()
        .with_child(
            Label::dynamic(|data: &AppState, _env| {
                data.diary_base_path
                    .as_deref()
                    .map(shorten_folder_path)
                    .unwrap_or_default()
            })
            .expand_width(),
        )
        .with_default_spacer()
        .with_child(build_select_folder_button())
        .with_default_spacer()
        .with_child(Label::new(text("page-login-recentFolders")))
        .with_default_spacer()
        .with_flex_child(build_recent_folders_list(), 1.0)
        .padding(COLUMN_PADDING)
}

fn build_open_button() -> impl Widget<AppState> {
    Button::new(text("page-login-start"))
        .on_click(|ctx, _data: &mut AppState, _| ctx.submit_command(druid_selector::FOLDER_OPEN))
        .disabled_if(|data, _| {
            data.diary_base_path.is_none()
                || (data.folder_protection == FolderProtection::Encrypted
                    && data.password.is_empty())
        })
        .expand_width()
        .fix_height(OPEN_BUTTON_HEIGHT)
}

/// Builds the right half of the page: the password input and the button that opens the folder.
fn build_password_column() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(build_password_section())
        .with_child(build_open_button())
        .padding(COLUMN_PADDING)
}

pub fn build_ui() -> impl Widget<AppState> {
    let label_welcome = Label::new(text("page-login-title"));
    let label_welcome_sub = Label::new(text("page-login-welcomeSub1"));

    Flex::column()
        .with_child(
            Image::new(diary_icon())
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
        .with_flex_child(
            Split::columns(build_folder_column(), build_password_column())
                .split_point(0.5)
                .bar_size(DIVIDER_WIDTH)
                .solid_bar(true)
                .min_size(COLUMN_MIN_WIDTH, COLUMN_MIN_WIDTH)
                .expand_height()
                .fix_width(PAGE_CONTENT_WIDTH),
            1.0,
        )
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .expand_height()
        .expand_width()
}
