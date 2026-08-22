use druid::{
    widget::{
        Button, CrossAxisAlignment, FillStrat, Flex, FlexParams, Image, Label, MainAxisAlignment,
        Scroll, TextBox, ViewSwitcher,
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

            Box::new(ListSelect::new(items).lens(DiaryBasePathLens)) as Box<dyn Widget<AppState>>
        },
    );

    Scroll::new(list_select)
        .vertical()
        .fix_height(RECENT_FOLDERS_LIST_HEIGHT)
}

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
                    Label::dynamic(|data: &AppState, env| {
                        if let Some(selected_path) = data.diary_base_path.clone() {
                            selected_path
                        } else {
                            "".to_string()
                        }
                    })
                    .expand_width(),
                    FlexParams::new(100_f64, CrossAxisAlignment::Start),
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
                                druid::commands::SHOW_OPEN_PANEL.with(dialog_options.clone()),
                            )
                        })
                        .expand(),
                    FlexParams::new(100_f64, CrossAxisAlignment::Start),
                )
                .with_default_spacer()
                .with_flex_child(
                    Label::new(LocalizedString::new("page-login-recentFolders")),
                    FlexParams::new(100_f64, CrossAxisAlignment::Start),
                )
                .with_child(build_recent_folders_list())
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
                    Button::new(LocalizedString::new("page-login-start"))
                        .on_click(|_, data: &mut AppState, _| data.page = AppPages::Diary)
                        .disabled_if(|data, _| data.diary_base_path.is_none())
                        .expand(),
                    FlexParams::new(100_f64, CrossAxisAlignment::Start),
                )
                .fix_width(400_f64)
                .expand_height(),
            FlexParams::new(60_f64, CrossAxisAlignment::Center),
        )
        .must_fill_main_axis(true)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_alignment(MainAxisAlignment::Start)
        .expand_height()
        .expand_width()
}
