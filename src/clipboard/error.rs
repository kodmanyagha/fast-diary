use thiserror::Error;

pub type ClipboardResult<T> = Result<T, ClipboardError>;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("The image on the clipboard could not be read: {0}")]
    UnreadableImage(#[source] image::ImageError),
    #[error("The bitmap on the clipboard has a damaged header.")]
    DamagedBitmapHeader,
}
