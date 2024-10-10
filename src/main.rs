#![windows_subsystem = "windows"]

pub mod modal;
pub mod utils;
pub mod view;

use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use chrono::{TimeDelta, Utc};
use druid::{
    AppLauncher, ExtEventSink, Point, Size, Target, WindowConfig, WindowDesc, WindowLevel,
    WindowSizePolicy,
};
use modal::{
    app_state::{AppPages, AppState, DiaryListItem},
    diary_datetime::DiaryDateTime,
};
use view::window::main::{self, main_window_controller::DIARY_ADD_ITEM};

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

    let main_window = WindowDesc::new(main::main_window::build_ui()).with_config(window_config);

    let mut app_data = AppState::new();
    fill_dummy_data(&mut app_data);

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
            DiaryListItem::new(
                create_dummy_time(-2),
                format!("counter: {}", counter).into(),
            ),
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

fn fill_dummy_data(app_data: &mut AppState) {
    let diaries = Arc::make_mut(&mut app_data.diaries);

    diaries.push(DiaryListItem::new(create_dummy_time(-1), "foo1".into()));
    diaries.push(DiaryListItem::new(create_dummy_time(-2), "bar1".into()));
    diaries.push(DiaryListItem::new(create_dummy_time(-3), "baz1".into()));
    diaries.push(DiaryListItem::new(create_dummy_time(-4), "foo2".into()));
    diaries.push(DiaryListItem::new(create_dummy_time(-5), "bar2".into()));
    diaries.push(DiaryListItem::new(create_dummy_time(-6), "baz2".into()));

    app_data.page = AppPages::Main;
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    struct Foo {
        pub id: String,
        pub id_static: &'static str,
    }

    impl Drop for Foo {
        fn drop(&mut self) {
            unsafe {
                // Rebuild String object which encapsulates original pointer of static str.
                // After that we can drop it.
                let reconstructed_string = String::from_raw_parts(
                    self.id_static.as_ptr() as *mut u8,
                    self.id_static.len(),
                    self.id_static.len(),
                );
                drop(reconstructed_string);
            }
            print_static_str(self.id_static);
        }
    }

    impl Foo {
        pub fn new(id: String) -> Self {
            // We must fix inner vec size as correct string length.
            let mut id_shrink = id.clone();
            id_shrink.shrink_to_fit();
            let id_static = id_shrink.clone().leak();

            Self { id, id_static }
        }
    }
    fn print_static_str(static_str: &'static str) {
        println!("static str: {}", static_str);
    }

    #[test]
    fn test_foo() {
        let foo1 = Foo::new("foo1".to_string());
        assert_eq!("foo1", foo1.id_static);
        drop(foo1);
    }

    #[test]
    fn test_static_str() {
        let string_1 = "string_1".to_string();
        let string_2 = "string_2".to_string();

        let static_str_1: &'static str = string_1.leak();
        let static_str_2: &'static str = string_2.leak();

        print_static_str(static_str_1);
        print_static_str(static_str_2);

        unsafe {
            let layout = std::alloc::Layout::new::<u8>();

            // let raw_ptr = static_str_1.get_unchecked_mut(0..static_str_1.len() - 1);
            // let raw_ptr = static_str_1.as_mut_ptr();
            std::alloc::dealloc(static_str_1.as_ptr() as *mut u8, layout);
            // std::mem::forget(*raw_ptr);
        }
        println!("Static str after dealloc");
        print_static_str(static_str_1);

        //assert_eq!(static_str_1, "string_1");
    }

    #[test]
    fn test_immediate_call_fn_1() {
        let mut str_1 = "str_1".to_string();

        let odd_result: anyhow::Result<String> = (|| {
            let now_ts = Utc::now().timestamp();
            str_1 = format!("ts: {}", now_ts.to_string());

            let x = str_1.parse::<i64>().map_err(|_| anyhow!("Can't parsed"))?;
            println!(">>> x: {x}");

            if now_ts % 2 == 0 {
                Err(anyhow!("Even number not accepted"))
            } else {
                Ok("Odd number is accepted".to_string())
            }
        })();
        println!(">>> str_1: {str_1}");

        if let Ok(odd) = odd_result {
            println!(">>> Success: {odd}");
        } else if let Err(err) = odd_result {
            println!(">>> Error: {err}");
        }

        // match odd_result {
        //     Ok(odd) => {
        //         println!(">>> Success: {odd}");
        //     }
        //     Err(err) => {
        //         println!(">>> Error: {err}");
        //     }
        // }
    }

    #[test]
    fn test_immediate_call_fn_2() {
        let mut str_1 = "str_1".to_string();

        let odd_option: Option<String> = (|| {
            str_1 = "str_1_1".to_string();
            let parsed_int = str_1.parse::<i32>().ok()?;

            println!("Parsed int: {parsed_int}");

            Some(parsed_int.to_string())
        })();

        if let Some(odd) = odd_option {
            println!(">>> Odd value: {odd}");
        } else {
            println!(">>> Odd is none returned.");
        }
    }
}
