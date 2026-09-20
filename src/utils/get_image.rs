use druid::ImageBuf;

const DIARY_ICON: &[u8] = include_bytes!("../../resources/images/diary_icon_1.png");

/// Returns the icon that the first screen shows. It is built into the program, so it does not
/// depend on the folder that the program is started from.
pub fn diary_icon() -> ImageBuf {
    ImageBuf::from_data(DIARY_ICON).unwrap_or_else(|err| {
        tracing::error!("An error occured when reading the diary icon: {err:?}");
        ImageBuf::empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_icon_can_be_decoded() {
        let icon = diary_icon();

        assert!(icon.width() > 0 && icon.height() > 0);
    }
}
