use std::sync::RwLock;

use anyhow::anyhow;
use chrono::Utc;

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

    // iflet is better than match syntax for Result and Option values.
    if let Ok(odd) = odd_result {
        println!(">>> Success: {odd}");
    } else if let Err(err) = odd_result {
        println!(">>> Error: {err}");
    }
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

#[test]
fn test_datetime_1() {
    let result: Option<()> = (|| {
        let date_str = "2020-04-12 22:10:57";
        let naive_datetime =
            chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").ok()?;

        println!(
            ">>> date 1: {}",
            naive_datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        );

        let date_str = "20-04-12 22:10:57";
        let naive_datetime =
            chrono::NaiveDateTime::parse_from_str(date_str, "%y-%m-%d %H:%M:%S").ok()?;

        println!(
            ">>> date 2: {}",
            naive_datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        );

        println!("{}:{} hello world", file!(), line!());

        Some(())
    })();

    println!("result:  {:?}", result);
}

#[test]
fn test_rwlock_1() {
    let rwlock_1 = RwLock::new(0);
    let mut writer = rwlock_1.write().unwrap();
    *writer += 1;

    // You have to drop the writer, otherwise thread enters to endless loop.
    drop(writer);

    let reader = rwlock_1.read().unwrap();
    println!(">>>>> Data: {}", *reader);
}

#[test]
fn test_str_chars() {
    let my_str: String = "ğüışöç".to_string();
    let my_str_ref = &my_str[0..];
    let chars = my_str_ref.chars();
    println!("chars: {:?}", chars);

    let sub_chars = chars.skip(1).take(2);
    println!("sub_chars: {:?}", sub_chars);

    let substr: String = sub_chars.collect();
    println!("substr: {}", substr);

    println!("substring method: {}", substring(&my_str, 1, 3));
}

fn substring(source: &str, from: usize, to: usize) -> String {
    if to <= from {
        return String::new();
    }
    source.chars().skip(from).take(to - from).collect()
}
