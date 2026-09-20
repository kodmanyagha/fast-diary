use druid::{
    widget::{Button, CrossAxisAlignment, Flex, FlexParams, Label, List, Scroll, Split},
    Color, Command, Insets, Target, Widget, WidgetExt,
};

use crate::{
    consts::druid_selector,
    modal::{
        app_state::{AppState, DiariesWithSelectionLens},
        state::app_pages::AppPages,
    },
    view::{
        page::diary::{
            diary_list_controller::DiaryListController,
            widgets::{
                btn_create_widget::build_ui::build_btn_create,
                build_diary_list_item::build_diary_list_item,
            },
        },
        widget::optional::optional,
    },
};

const STATUS_COLOR: Color = Color::rgb8(200, 60, 60);

pub fn build_ui() -> impl Widget<AppState> {
    let btn_create_1 = build_btn_create();
    let btn_settings = Button::new("Settings")
        .on_click(|_ctx, data: &mut AppState, _| {
            data.status_message.clear();
            data.page = AppPages::Settings;
        })
        .expand();
    let btn_close = Button::new("Close")
        .on_click(|ctx, _data: &mut AppState, _| ctx.submit_command(druid_selector::FOLDER_LOCK))
        .expand();

    let split_left_side = Flex::column()
        .with_flex_child(
            Flex::row()
                .with_flex_child(
                    btn_create_1,
                    FlexParams::new(100.0, Some(CrossAxisAlignment::Center)),
                )
                .with_flex_child(
                    btn_settings,
                    FlexParams::new(70.0, Some(CrossAxisAlignment::Center)),
                )
                .with_flex_child(
                    btn_close,
                    FlexParams::new(70.0, Some(CrossAxisAlignment::Center)),
                )
                .expand_width(),
            FlexParams::new(10.0, Some(CrossAxisAlignment::Center)),
        )
        .with_child(
            Label::dynamic(|data: &AppState, _env| data.status_message.clone())
                .with_text_color(STATUS_COLOR),
        )
        .with_flex_child(
            Scroll::new(List::new(build_diary_list_item).lens(DiariesWithSelectionLens))
                .vertical()
                .expand_width()
                .expand_height()
                .controller(DiaryListController::new()),
            FlexParams::new(90.0, Some(CrossAxisAlignment::Start)),
        )
        .expand_width()
        .expand_height()
        .padding(Insets::uniform(10.0));

    let split_right_side = Flex::column()
        .with_flex_child(
            optional(),
            FlexParams::new(100.0, CrossAxisAlignment::Center),
        )
        .expand_width()
        .expand_height()
        .padding(Insets::uniform(10.0));

    Flex::column()
        .with_flex_child(
            Split::columns(split_left_side, split_right_side)
                .split_point(0.3)
                .draggable(true)
                .bar_size(3f64)
                .solid_bar(true)
                .min_size(150f64, 400f64)
                .expand_width()
                .expand_height(),
            FlexParams::new(100.0, Some(CrossAxisAlignment::Start)),
        )
        .expand_height()
        .expand_width()
        .on_added(|_widget, ctx, _data, _env| {
            ctx.submit_command(Command::new(
                druid_selector::DIARY_LOAD_FOLDER,
                (),
                Target::Global,
            ));
        })
}
