use std::path;

use druid::ImageBuf;

pub fn get_image(path: &str) -> ImageBuf {
    let img_buf = ImageBuf::from_file(path::absolute(path).unwrap());

    match img_buf {
        Ok(img_data) => img_data,
        Err(err) => {
            log::error!("An error occured when reading image: {:?}", err);
            ImageBuf::empty()
        }
    }
}
