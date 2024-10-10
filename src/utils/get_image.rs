use std::path::{absolute, PathBuf};

use druid::ImageBuf;

pub fn get_image(path: &str) -> ImageBuf {
    println!(
        ">>> CWD: {} --- {}",
        std::env::current_dir().unwrap().to_str().unwrap(),
        absolute(path).unwrap().to_str().unwrap()
    );

    let img_buf = ImageBuf::from_file(absolute(path).unwrap());

    match img_buf {
        Ok(img_data) => img_data,
        Err(err) => {
            log::error!("An error occured when reading image: {:?}", err);
            ImageBuf::empty()
        }
    }
}
