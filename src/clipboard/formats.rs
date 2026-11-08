use super::clipboard_image::ClipboardImageEncoding;

/// How a list of copied files is written in a clipboard format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileListEncoding {
    /// One URI or path per line, as in `text/uri-list`.
    UriList,
    /// GNOME's `x-special/gnome-copied-files`: a `copy` or `cut` line followed by URIs.
    GnomeCopiedFiles,
    /// The Windows `DROPFILES` structure of `CF_HDROP`.
    DroppedFileList,
}

/// The clipboard formats that carry copied files on this platform, most preferred first.
#[cfg(target_os = "linux")]
pub const FILE_LIST_FORMATS: &[(&str, FileListEncoding)] = &[
    ("text/uri-list", FileListEncoding::UriList),
    (
        "x-special/gnome-copied-files",
        FileListEncoding::GnomeCopiedFiles,
    ),
];
#[cfg(windows)]
pub const FILE_LIST_FORMATS: &[(&str, FileListEncoding)] =
    &[("CF_HDROP", FileListEncoding::DroppedFileList)];
#[cfg(target_os = "macos")]
pub const FILE_LIST_FORMATS: &[(&str, FileListEncoding)] =
    &[("public.file-url", FileListEncoding::UriList)];
#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
pub const FILE_LIST_FORMATS: &[(&str, FileListEncoding)] = &[];

/// The clipboard formats that carry image data on this platform.
#[cfg(target_os = "linux")]
pub const IMAGE_FORMATS: &[(&str, ClipboardImageEncoding)] = &[
    ("image/png", ClipboardImageEncoding::File),
    ("image/jpeg", ClipboardImageEncoding::File),
    ("image/webp", ClipboardImageEncoding::File),
    ("image/gif", ClipboardImageEncoding::File),
    ("image/bmp", ClipboardImageEncoding::File),
    ("image/tiff", ClipboardImageEncoding::File),
];
#[cfg(windows)]
pub const IMAGE_FORMATS: &[(&str, ClipboardImageEncoding)] = &[
    ("PNG", ClipboardImageEncoding::File),
    ("CF_DIB", ClipboardImageEncoding::WindowsBitmap),
];
#[cfg(target_os = "macos")]
pub const IMAGE_FORMATS: &[(&str, ClipboardImageEncoding)] = &[
    ("public.png", ClipboardImageEncoding::File),
    ("public.tiff", ClipboardImageEncoding::File),
];
#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
pub const IMAGE_FORMATS: &[(&str, ClipboardImageEncoding)] = &[];
