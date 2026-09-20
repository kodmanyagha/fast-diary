#[derive(Default)]
pub struct AppConfig {
    pub window_position: (usize, usize),
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
    let mut config = AppConfig::new();

    config.window_position = (150, 150);

    config
}
