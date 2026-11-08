use std::path::{Path, PathBuf};

use super::{
    clipboard_image::{ClipboardImage, ClipboardImageEncoding},
    dropped_file_list::paths_of_dropped_file_list,
    formats::{FileListEncoding, FILE_LIST_FORMATS, IMAGE_FORMATS},
    path_text::paths_in_text,
};
use crate::storage::asset_name::image_extension;

/// What a paste into the diary editor has to do with the content of the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteContent {
    /// The clipboard holds text that is pasted as it is.
    Text,
    /// Files were copied, or the copied text names image files; their images are inserted.
    Files(Vec<PathBuf>),
    /// The clipboard holds image data, such as a screenshot, that is saved and inserted.
    Image(ClipboardImage),
    /// The clipboard holds something that cannot go into a diary, such as a video.
    Unsupported,
    /// The clipboard is empty.
    Nothing,
}

/// Read access to a clipboard, so that the decision about a paste can be made, and tested,
/// without the clipboard of the system.
pub trait ClipboardSource {
    /// Returns one of `formats` that the clipboard holds, if it holds any of them.
    fn first_available(&self, formats: &[&'static str]) -> Option<&'static str>;
    /// Returns the content of the clipboard in `format`.
    fn read_format(&self, format: &'static str) -> Option<Vec<u8>>;
    /// Returns the text on the clipboard.
    fn read_text(&self) -> Option<String>;
    /// Tells whether the clipboard holds nothing at all.
    fn is_empty(&self) -> bool;
}

impl ClipboardSource for druid::Clipboard {
    fn first_available(&self, formats: &[&'static str]) -> Option<&'static str> {
        self.preferred_format(formats)
    }

    fn read_format(&self, format: &'static str) -> Option<Vec<u8>> {
        self.get_format(format)
    }

    fn read_text(&self) -> Option<String> {
        self.get_string()
    }

    fn is_empty(&self) -> bool {
        self.available_type_names().is_empty()
    }
}

/// Reads the paths of a list of copied files written in `encoding`.
fn paths_of_file_list(
    bytes: &[u8],
    encoding: FileListEncoding,
    home: Option<&Path>,
) -> Option<Vec<PathBuf>> {
    let text = || {
        std::str::from_utf8(bytes)
            .ok()
            .map(|text| text.trim_end_matches('\0'))
    };

    match encoding {
        FileListEncoding::UriList => text().and_then(|uris| paths_in_text(uris, home)),
        FileListEncoding::GnomeCopiedFiles => text()
            .and_then(|content| content.split_once('\n'))
            .and_then(|(_operation, uris)| paths_in_text(uris, home)),
        FileListEncoding::DroppedFileList => paths_of_dropped_file_list(bytes),
    }
}

/// Returns the files that were copied onto the clipboard, if any.
fn copied_files(
    source: &impl ClipboardSource,
    formats: &[(&'static str, FileListEncoding)],
    home: Option<&Path>,
) -> Option<Vec<PathBuf>> {
    formats.iter().find_map(|(format, encoding)| {
        source
            .first_available(&[format])
            .and_then(|format| source.read_format(format))
            .and_then(|bytes| paths_of_file_list(&bytes, *encoding, home))
            .filter(|paths| !paths.is_empty())
    })
}

/// Returns the image files that `text` names, when it consists of nothing but paths of
/// existing images.
fn image_files_named_by(text: &str, home: Option<&Path>) -> Option<Vec<PathBuf>> {
    paths_in_text(text, home).filter(|paths| {
        paths
            .iter()
            .all(|path| path.is_file() && image_extension(path).is_some())
    })
}

/// Returns what the text on the clipboard stands for, if there is any text.
fn copied_text(source: &impl ClipboardSource, home: Option<&Path>) -> Option<PasteContent> {
    source
        .read_text()
        .filter(|text| !text.trim().is_empty())
        .map(|text| {
            image_files_named_by(&text, home).map_or(PasteContent::Text, PasteContent::Files)
        })
}

/// Returns the image data on the clipboard, if any.
fn copied_image(
    source: &impl ClipboardSource,
    formats: &[(&'static str, ClipboardImageEncoding)],
) -> Option<ClipboardImage> {
    let names = formats.iter().map(|(name, _)| *name).collect::<Vec<_>>();
    let format = source.first_available(&names)?;
    let encoding = formats
        .iter()
        .find(|(name, _)| *name == format)
        .map(|(_, encoding)| *encoding)?;

    source
        .read_format(format)
        .filter(|bytes| !bytes.is_empty())
        .map(|bytes| ClipboardImage::new(encoding, bytes))
}

/// Decides what a paste does with `source`, looking for copied files first, then for text,
/// which wins over image data because office programs put a picture of copied text on the
/// clipboard too, and for image data last.
fn classify_with(
    source: &impl ClipboardSource,
    home: Option<&Path>,
    file_list_formats: &[(&'static str, FileListEncoding)],
    image_formats: &[(&'static str, ClipboardImageEncoding)],
) -> PasteContent {
    copied_files(source, file_list_formats, home)
        .map(PasteContent::Files)
        .or_else(|| copied_text(source, home))
        .or_else(|| copied_image(source, image_formats).map(PasteContent::Image))
        .unwrap_or_else(|| {
            if source.is_empty() {
                PasteContent::Nothing
            } else {
                PasteContent::Unsupported
            }
        })
}

/// Decides what a paste does with the content of `source`, using the clipboard formats of this
/// platform. `home` is where paths written as `~/...` start.
pub fn classify_paste(source: &impl ClipboardSource, home: Option<&Path>) -> PasteContent {
    classify_with(source, home, FILE_LIST_FORMATS, IMAGE_FORMATS)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs};

    use super::*;

    const URI_LIST: &str = "text/uri-list";
    const GNOME_FILES: &str = "x-special/gnome-copied-files";
    const PNG: &str = "image/png";
    const BITMAP: &str = "CF_DIB";
    const VIDEO: &str = "video/mp4";

    const FILE_FORMATS: &[(&str, FileListEncoding)] = &[
        (URI_LIST, FileListEncoding::UriList),
        (GNOME_FILES, FileListEncoding::GnomeCopiedFiles),
    ];
    const IMAGE_FORMATS: &[(&str, ClipboardImageEncoding)] = &[
        (PNG, ClipboardImageEncoding::File),
        (BITMAP, ClipboardImageEncoding::WindowsBitmap),
    ];

    #[derive(Default)]
    struct FakeClipboard {
        formats: HashMap<&'static str, Vec<u8>>,
        text: Option<String>,
    }

    impl FakeClipboard {
        fn with_format(mut self, format: &'static str, bytes: &[u8]) -> Self {
            self.formats.insert(format, bytes.to_vec());
            self
        }

        fn with_text(mut self, text: &str) -> Self {
            self.text = Some(text.to_string());
            self
        }
    }

    impl ClipboardSource for FakeClipboard {
        fn first_available(&self, formats: &[&'static str]) -> Option<&'static str> {
            formats
                .iter()
                .find(|format| self.formats.contains_key(*format))
                .copied()
        }

        fn read_format(&self, format: &'static str) -> Option<Vec<u8>> {
            self.formats.get(format).cloned()
        }

        fn read_text(&self) -> Option<String> {
            self.text.clone()
        }

        fn is_empty(&self) -> bool {
            self.formats.is_empty() && self.text.is_none()
        }
    }

    fn classify(clipboard: &FakeClipboard) -> PasteContent {
        classify_with(clipboard, None, FILE_FORMATS, IMAGE_FORMATS)
    }

    #[test]
    fn copied_files_come_first_whatever_else_is_on_the_clipboard() {
        let clipboard = FakeClipboard::default()
            .with_format(URI_LIST, b"file:///tmp/a.png\r\nfile:///tmp/notes.txt\r\n")
            .with_format(PNG, b"thumbnail")
            .with_text("/tmp/a.png");

        assert_eq!(
            classify(&clipboard),
            PasteContent::Files(vec![
                PathBuf::from("/tmp/a.png"),
                PathBuf::from("/tmp/notes.txt")
            ])
        );
    }

    #[test]
    fn gnome_file_lists_skip_their_operation_line() {
        let clipboard =
            FakeClipboard::default().with_format(GNOME_FILES, b"copy\nfile:///tmp/a%20b.png");

        assert_eq!(
            classify(&clipboard),
            PasteContent::Files(vec![PathBuf::from("/tmp/a b.png")])
        );
    }

    #[test]
    fn text_naming_existing_images_inserts_them() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let image = folder.path().join("Fotoğraf.png");
        fs::write(&image, b"png")?;
        let clipboard = FakeClipboard::default().with_text(&format!("\"{}\"", image.display()));

        assert_eq!(classify(&clipboard), PasteContent::Files(vec![image]));
        Ok(())
    }

    #[test]
    fn text_naming_missing_files_or_other_files_stays_text() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let notes = folder.path().join("notes.txt");
        fs::write(&notes, b"notes")?;

        let missing = FakeClipboard::default().with_text("/no/such/picture.png");
        let not_an_image = FakeClipboard::default().with_text(&notes.to_string_lossy());

        assert_eq!(classify(&missing), PasteContent::Text);
        assert_eq!(classify(&not_an_image), PasteContent::Text);
        Ok(())
    }

    #[test]
    fn text_wins_over_the_picture_that_office_programs_add() {
        let clipboard = FakeClipboard::default()
            .with_text("Merhaba dünya")
            .with_format(PNG, b"picture of the text");

        assert_eq!(classify(&clipboard), PasteContent::Text);
    }

    #[test]
    fn image_data_without_text_is_an_image() {
        let clipboard = FakeClipboard::default()
            .with_format(BITMAP, b"bitmap")
            .with_text("  ");

        assert_eq!(
            classify(&clipboard),
            PasteContent::Image(ClipboardImage::new(
                ClipboardImageEncoding::WindowsBitmap,
                b"bitmap".to_vec()
            ))
        );
    }

    #[test]
    fn videos_and_other_data_are_unsupported_and_nothing_is_nothing() {
        let video = FakeClipboard::default().with_format(VIDEO, b"mp4");

        assert_eq!(classify(&video), PasteContent::Unsupported);
        assert_eq!(classify(&FakeClipboard::default()), PasteContent::Nothing);
    }
}
