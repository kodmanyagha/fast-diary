#![windows_subsystem = "windows"]

pub mod modal;
pub mod view;

use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use chrono::{DateTime, TimeDelta, Utc};
use druid::{
    AppLauncher, ExtEventSink, Point, Selector, Size, Target, WindowConfig, WindowDesc,
    WindowLevel, WindowSizePolicy,
};
use im::{HashMap, OrdMap};
use modal::{
    app_data::{AppData, AppPages, DiaryListItem},
    diary_datetime::DiaryDateTime,
    selector::ADD_DIARY_ITEM,
};
use tracing::{instrument::WithSubscriber, Dispatch};
use view::window::main;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().expect(".env file not found.");
    env_logger::init();

    let window_config = WindowConfig::default()
        .window_size_policy(WindowSizePolicy::User)
        .set_level(WindowLevel::AppWindow)
        .set_position(Point::new(1750f64, 500f64))
        .with_min_size(Size::new(600f64, 400f64))
        .window_size(Size::new(800f64, 600f64));

    let main_window = WindowDesc::new(main::build_ui()).with_config(window_config);

    let mut app_data = AppData::new();
    fill_dummy_data(&mut app_data);

    let app = AppLauncher::with_window(main_window);

    tokio::spawn(event_sink_handle(app.get_external_handle()));

    app.log_to_console()
        .launch(app_data)
        .map_err(|err| anyhow!(err.to_string()))
}

async fn event_sink_handle(event_sink: ExtEventSink) {
    event_sink.add_idle_callback(move |win_handle: &mut AppData| {
        //
    });

    let mut counter = 0;
    loop {
        let _ = event_sink.submit_command(
            ADD_DIARY_ITEM,
            DiaryListItem::new(create_dummy_time(-2), format!("counter: {}", counter)),
            Target::Global,
        );

        tokio::time::sleep(Duration::from_millis(3333)).await;
        counter += 1;
    }
}

fn create_dummy_time(seconds: i64) -> DiaryDateTime<Utc> {
    Utc::now()
        .checked_add_signed(TimeDelta::seconds(seconds))
        .unwrap_or_default()
        .into()
}

fn fill_dummy_data(app_data: &mut AppData) {
    if let Some(diaries) = Arc::get_mut(&mut app_data.diaries) {
        diaries.push(DiaryListItem::new(
            create_dummy_time(-1),
            "foo1".to_string(),
        ));
        diaries.push(DiaryListItem::new(
            create_dummy_time(-2),
            "bar1".to_string(),
        ));
        diaries.push(DiaryListItem::new(
            create_dummy_time(-3),
            "baz1".to_string(),
        ));
        diaries.push(DiaryListItem::new(
            create_dummy_time(-4),
            "foo2".to_string(),
        ));
        diaries.push(DiaryListItem::new(
            create_dummy_time(-5),
            "bar2".to_string(),
        ));
        diaries.push(DiaryListItem::new(
            create_dummy_time(-6),
            "baz2".to_string(),
        ));
    }

    app_data.page = AppPages::Diary;
}
