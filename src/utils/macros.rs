/// giver: Give Result Data. Gently unwraps the data if it Ok, otherwise returns the Err.
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

/// giveo: Give Option Data. Gently unwraps the data if it Some, otherwise returns empty value.
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
