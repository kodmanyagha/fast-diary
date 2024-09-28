#![windows_subsystem = "windows"]

pub mod modal;
pub mod view;

use anyhow::anyhow;
use druid::{AppLauncher, WindowDesc};
use modal::app_data::AppData;
use tracing::{instrument::WithSubscriber, Dispatch};
use view::main_window;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let main_window = WindowDesc::new(main_window::build_ui());

    let data = AppData::new();

    let app = AppLauncher::with_window(main_window);
    // let app = app.localization_resources(vec!["app".to_string()], "lang".to_string());

    let external_event = app.get_external_handle();
    external_event.with_subscriber(Dispatch::default());

    app.log_to_console()
        .launch(data)
        .map_err(|err| anyhow!(err.to_string()))
}
