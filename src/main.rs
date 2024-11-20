#![windows_subsystem = "windows"]

pub mod config;
pub mod modal;
pub mod utils;
pub mod view;

use std::time::Duration;

use anyhow::anyhow;
use chrono::{TimeDelta, Utc};
use config::app_config::get_app_config;
use druid::{
    AppLauncher, ExtEventSink, Point, Size, Target, WindowConfig, WindowDesc, WindowLevel,
    WindowSizePolicy,
};
use modal::{
    app_state::AppState, diary_datetime::DiaryDateTime, state::diary_list_item::DiaryListItem,
};
use view::window::main::{self, main_window_controller::DIARY_ADD_ITEM};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv().ok();
    env_logger::try_init()?;

    let app_config = get_app_config();
    // TODO Log or print not working in here, fix this problem.
    log::error!(">>>>>>>>>>>>> Current os: {}", std::env::consts::OS);

    let window_config = WindowConfig::default()
        .window_size_policy(WindowSizePolicy::User)
        .set_level(WindowLevel::AppWindow)
        .set_position(Point::new(
            app_config.window_position.0 as f64,
            app_config.window_position.1 as f64,
        ))
        .with_min_size(Size::new(600f64, 400f64))
        .window_size(Size::new(800f64, 600f64));

    let app_data = AppState::new();
    let main_window = WindowDesc::new(main::main_window::build_ui()).with_config(window_config);
    let app = AppLauncher::with_window(main_window);

    tokio::spawn(event_sink_handle(app.get_external_handle()));

    app.log_to_console()
        .launch(app_data)
        .map_err(|err| anyhow!(err.to_string()))
}

async fn event_sink_handle(event_sink: ExtEventSink) {
    event_sink.add_idle_callback(move |_data: &mut AppState| {
        //
    });

    let mut counter = 0;
    loop {
        if counter >= 0 {
            break;
        }

        let _ = event_sink.submit_command(
            DIARY_ADD_ITEM,
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
