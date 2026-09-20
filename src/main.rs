#![windows_subsystem = "windows"]

use std::time::Duration;

use anyhow::anyhow;
use chrono::{TimeDelta, Utc};
use druid::{
    AppLauncher, ExtEventSink, Point, Target, WindowConfig, WindowDesc, WindowLevel,
    WindowSizePolicy, WindowState,
};
use fast_diary::{
    config::{
        app_config::get_app_config,
        settings::Settings,
        window_settings::{WindowGeometry, MIN_WINDOW_SIZE},
    },
    consts::druid_selector,
    modal::{
        app_state::AppState, diary_datetime::DiaryDateTime, state::diary_list_item::DiaryListItem,
    },
    utils::{event_sink::set_event_sink, logger::init_tracing_subscriber},
    view::window::main::{
        self, main_menu::build_main_menu, main_window_delegate::MainWindowDelegate,
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv().ok();
    init_tracing_subscriber();

    let app_config = get_app_config();
    tracing::info!("Current os: {}", std::env::consts::OS);

    let settings = Settings::load();
    let launch_plan = settings.window.launch_plan();
    let default_position = Point::new(
        app_config.window_position.0 as f64,
        app_config.window_position.1 as f64,
    );
    let window_config = WindowConfig::default()
        .window_size_policy(WindowSizePolicy::User)
        .set_level(WindowLevel::AppWindow)
        .set_position(launch_plan.position.unwrap_or(default_position))
        .with_min_size(MIN_WINDOW_SIZE)
        .window_size(launch_plan.size)
        .set_window_state(if launch_plan.maximized {
            WindowState::Maximized
        } else {
            WindowState::Restored
        });
    let requested_geometry = launch_plan
        .position
        .filter(|_| !launch_plan.maximized)
        .map(|position| WindowGeometry::new(position, launch_plan.size));

    let mut app_data = AppState::new();
    app_data.recent_folders = settings.recent_folders.into();
    app_data.diary_view_mode = settings.diary_view_mode;
    app_data.editor_mode = settings.editor_mode;
    app_data.language = settings.language;
    app_data.window = settings.window;
    app_data.list_split_ratio = settings.layout.list_split_ratio();
    app_data.editor_split_ratio = settings.layout.editor_split_ratio();
    let main_window = WindowDesc::new(main::main_window::build_ui())
        .with_config(window_config)
        .menu(build_main_menu);
    let app = AppLauncher::with_window(main_window).delegate(MainWindowDelegate::new(
        requested_geometry,
        default_position,
    ));

    set_event_sink(app.get_external_handle());
    tokio::spawn(event_sink_handle(app.get_external_handle()));

    app.launch(app_data).map_err(|err| anyhow!(err.to_string()))
}

async fn event_sink_handle(event_sink: ExtEventSink) {
    event_sink.add_idle_callback(move |app_state: &mut AppState| {
        tracing::info!("Event sink cb: {}", app_state.app_title);
    });

    let mut counter = 0;
    loop {
        if counter >= 0 {
            break;
        }

        let _ = event_sink.submit_command(
            druid_selector::DIARY_ADD_ITEM,
            DiaryListItem::new()
                .with_date(create_dummy_time(-2))
                .with_summary(format!("counter: {}", counter)),
            Target::Global,
        );

        if counter % 20 == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        counter += 1;
    }
}

fn create_dummy_time(seconds: i64) -> DiaryDateTime<Utc> {
    Utc::now()
        .checked_add_signed(TimeDelta::seconds(seconds))
        .unwrap_or_default()
        .into()
}
