use druid::{
    widget::{CrossAxisAlignment, Flex, FlexParams, Label, List, Scroll, ViewSwitcher},
    Color, Command, Insets, Target, Widget, WidgetExt,
};

use crate::{
    consts::druid_selector,
    modal::{
        app_state::{AppState, DiariesWithSelectionLens},
        state::diary_view_mode::DiaryViewMode,
    },
    view::{
        page::diary::{
            diary_list_controller::DiaryListController,
            widgets::{
                build_diary_list_item::build_diary_list_item, icon_toolbar::build_icon_toolbar,
            },
        },
        widget::{calendar::build_calendar, optional::optional, persistent_split::PersistentSplit},
    },
};

const STATUS_COLOR: Color = Color::rgb8(200, 60, 60);
const TOOLBAR_SPACING: f64 = 8.0;

fn build_diary_list() -> impl Widget<AppState> {
    Scroll::new(List::new(build_diary_list_item).lens(DiariesWithSelectionLens))
        .vertical()
        .expand_width()
        .expand_height()
        .controller(DiaryListController::new())
}

fn build_diary_browser(mode: DiaryViewMode) -> Box<dyn Widget<AppState>> {
    match mode {
        DiaryViewMode::List => Box::new(build_diary_list()),
        DiaryViewMode::Calendar => Box::new(build_calendar()),
    }
}

pub fn build_ui() -> impl Widget<AppState> {
    let split_left_side = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(build_icon_toolbar())
        .with_spacer(TOOLBAR_SPACING)
        .with_child(
            Label::dynamic(|data: &AppState, _env| data.status_message.clone())
                .with_text_color(STATUS_COLOR),
        )
        .with_flex_child(
            ViewSwitcher::new(
                |data: &AppState, _env| data.diary_view_mode,
                |mode, _data, _env| build_diary_browser(*mode),
            ),
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
            PersistentSplit::columns(
                split_left_side,
                split_right_side,
                AppState::list_split_ratio,
            )
            .bar_size(3f64)
            .min_widths(200f64, 400f64)
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
