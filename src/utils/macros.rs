#[macro_export]
macro_rules! give {
    ($result:expr) => {
        match $result {
            Ok(item) => item,
            Err(_) => return,
        }
    };

    ($option:expr) => {
        match $option {
            Some(item) => item,
            None => return,
        }
    };
}
