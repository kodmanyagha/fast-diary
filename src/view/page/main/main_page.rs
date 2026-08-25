use druid::{
    widget::{
        Button, CrossAxisAlignment, FillStrat, Flex, Image, Label, MainAxisAlignment, Scroll,
        TextBox, ViewSwitcher,
    },
    FileDialogOptions, LocalizedString, Widget, WidgetExt,
};
use druid_widget_nursery::ListSelect;

use crate::{
    modal::{
        app_state::{AppState, DiaryBasePathLens, OpenFilePurpose},
        state::app_pages::AppPages,
    },
    utils::get_image::get_image,
};

const RECENT_FOLDERS_LIST_HEIGHT: f64 = 120_f64;

/// Rebuilds a `ListSelect` whenever `recent_folders` changes, since
/// `ListSelect` takes its list of choices at construction time rather than
/// reading it reactively from the widget data.
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
                .with_child(Label::new(LocalizedString::new("page-login-enterPassword")))
                .with_default_spacer()
                .with_child(TextBox::new().expand_width().lens(AppState::password))
                .with_default_spacer()
                .with_child(
                    Button::new(LocalizedString::new("page-login-start"))
                        .on_click(|_, data: &mut AppState, _| data.page = AppPages::Diary)
                        .disabled_if(|data, _| data.diary_base_path.is_none())
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
