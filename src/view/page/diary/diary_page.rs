use druid::{
    widget::{CrossAxisAlignment, Flex, FlexParams, List, Scroll, Split},
    Command, Insets, Target, Widget, WidgetExt,
};

use crate::{
    consts::druid_selector,
    modal::app_state::{AppState, DiariesWithSelectionLens},
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

pub fn build_ui() -> impl Widget<AppState> {
    // tracing::info!(">>> diary::build_ui() invoked");
    let btn_create_1 = build_btn_create();

    let split_left_side = Flex::column()
        .with_flex_child(
            Flex::row()
                .with_flex_child(
                    btn_create_1,
                    FlexParams::new(100.0, Some(CrossAxisAlignment::Center)),
                )
                .expand_width(),
            FlexParams::new(10.0, Some(CrossAxisAlignment::Center)),
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
        .on_added(|widget, ctx, data, env| {
            ctx.submit_command(Command::new(
                druid_selector::DIARY_LOAD_FOLDER,
                (),
                Target::Global,
            ));
        })
}
