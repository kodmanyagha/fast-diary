#[macro_export]
macro_rules! giver {
    ($result:expr) => {
        match $result {
            Ok(item) => item,
            Err(_) => return,
        }
    };
}

#[macro_export]
macro_rules! giveo {
    ($option:expr) => {
        match $option {
            Some(item) => item,
            None => return,
        }
    };

    ($option:expr, $ret:expr) => {
        match $option {
            Some(item) => item,
            None => return Err($ret),
        }
    };
}
