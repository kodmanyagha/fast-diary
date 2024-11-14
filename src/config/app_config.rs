#[derive(Default)]
pub struct AppConfig {
    pub window_positionn: (usize, usize),
    pub window_size: (usize, usize),
}

impl AppConfig {
    pub fn new() -> Self {
        AppConfig {
            ..Default::default()
        }
    }
}

pub fn get_app_config() -> AppConfig {
    let config = AppConfig::new();
    // buraya dön

    config
}
