use druid::{
    widget::{CrossAxisAlignment, Flex, FlexParams, List, Scroll, Split},
    Command, Insets, Target, Widget, WidgetExt,
};

use crate::{
    modal::app_state::AppState,
    view::{widget::optional::optional, window::main::main_window_controller::DIARY_LOAD_FOLDER},
};

use super::widgets::diary_list_item::create_diary_list_item;

pub fn build_ui() -> impl Widget<AppState> {
    log::info!(">>> diary::build_ui() invoked");

    Flex::column()
        .with_flex_child(
            Split::columns(
                Flex::column()
                    .with_flex_child(
                        Scroll::new(List::new(create_diary_list_item).lens(AppState::diaries))
                            .vertical()
                            .expand_width()
                            .expand_height(),
                        FlexParams::new(90.0, Some(CrossAxisAlignment::Start)),
                    )
                    .expand_width()
                    .expand_height()
                    .padding(Insets::uniform(10.0)),
                Flex::column()
                    .with_flex_child(
                        optional(),
                        FlexParams::new(100.0, CrossAxisAlignment::Center),
                    )
                    .expand_width()
                    .expand_height()
                    .padding(Insets::uniform(10.0)),
            )
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
            ctx.submit_command(Command::new(DIARY_LOAD_FOLDER, (), Target::Global));
        })
}
