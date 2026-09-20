#[macro_export]
macro_rules! giver {
    ($result:expr) => {
        match $result {
            Ok(item) => item,
            Err(_) => return,
        }
    };

    ($result:expr, $ret:expr) => {
        match $result {
            Ok(item) => item,
            Err(_) => return Err($ret),
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
