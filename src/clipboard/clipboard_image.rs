use std::io::Cursor;

use image::ImageOutputFormat;

use super::error::{ClipboardError, ClipboardResult};

const BITMAP_FILE_HEADER_SIZE: u32 = 14;
const BITMAP_INFO_HEADER_SIZE: u32 = 40;
const BITFIELDS_COMPRESSION: u32 = 3;
const BITFIELDS_MASKS_SIZE: u32 = 12;
const COLOR_TABLE_ENTRY_SIZE: u32 = 4;
const MAX_INDEXED_BIT_COUNT: u16 = 8;

/// How the bytes of an image on the clipboard are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardImageEncoding {
    /// A complete image file, such as PNG, JPEG or TIFF.
    File,
    /// A Windows device-independent bitmap (`CF_DIB`): a BMP file without its file header.
    WindowsBitmap,
}

/// An image taken from the clipboard, as the clipboard handed it over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardImage {
    pub encoding: ClipboardImageEncoding,
    pub bytes: Vec<u8>,
}

impl ClipboardImage {
    pub fn new(encoding: ClipboardImageEncoding, bytes: Vec<u8>) -> Self {
        Self { encoding, bytes }
    }

    /// Converts the image into PNG, so that every pasted image is kept in the same format
    /// whatever the clipboard of the platform offered.
    pub fn to_png(&self) -> ClipboardResult<Vec<u8>> {
        let file = match self.encoding {
            ClipboardImageEncoding::File => self.bytes.clone(),
            ClipboardImageEncoding::WindowsBitmap => {
                bitmap_file_of(&self.bytes).ok_or(ClipboardError::DamagedBitmapHeader)?
            }
        };

        let mut png = Cursor::new(Vec::new());
        image::load_from_memory(&file)
            .and_then(|decoded| decoded.write_to(&mut png, ImageOutputFormat::Png))
            .map_err(ClipboardError::UnreadableImage)?;
        Ok(png.into_inner())
    }
}

fn u16_at(bytes: &[u8], position: usize) -> Option<u16> {
    bytes
        .get(position..position + 2)
        .and_then(|field| field.try_into().ok())
        .map(u16::from_le_bytes)
}

fn u32_at(bytes: &[u8], position: usize) -> Option<u32> {
    bytes
        .get(position..position + 4)
        .and_then(|field| field.try_into().ok())
        .map(u32::from_le_bytes)
}

/// Returns where the pixels of the device-independent bitmap `bitmap` start, counted from the
/// start of the BMP file that has it.
fn pixel_offset_of(bitmap: &[u8]) -> Option<u32> {
    let header_size = u32_at(bitmap, 0)?;
    let bit_count = u16_at(bitmap, 14)?;
    let compression = u32_at(bitmap, 16)?;
    let used_colors = u32_at(bitmap, 32)?;

    let color_count = match (used_colors, bit_count) {
        (0, bits) if bits <= MAX_INDEXED_BIT_COUNT => 1u32 << bits,
        (used, _) => used,
    };
    let masks_size =
        if compression == BITFIELDS_COMPRESSION && header_size == BITMAP_INFO_HEADER_SIZE {
            BITFIELDS_MASKS_SIZE
        } else {
            0
        };

    BITMAP_FILE_HEADER_SIZE
        .checked_add(header_size)?
        .checked_add(masks_size)?
        .checked_add(color_count.checked_mul(COLOR_TABLE_ENTRY_SIZE)?)
}

/// Puts the BMP file header in front of the device-independent bitmap `bitmap`, turning it
/// into a BMP file that image decoders understand.
fn bitmap_file_of(bitmap: &[u8]) -> Option<Vec<u8>> {
    let pixel_offset = pixel_offset_of(bitmap)?;
    let file_size = u32::try_from(bitmap.len())
        .ok()?
        .checked_add(BITMAP_FILE_HEADER_SIZE)?;

    Some(
        [
            b"BM".as_slice(),
            &file_size.to_le_bytes(),
            &0u32.to_le_bytes(),
            &pixel_offset.to_le_bytes(),
            bitmap,
        ]
        .concat(),
    )
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    use super::*;

    fn sample_image() -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(3, 2, Rgba([10, 20, 30, 255])))
    }

    fn encoded(format: ImageOutputFormat) -> anyhow::Result<Vec<u8>> {
        let mut bytes = Cursor::new(Vec::new());
        sample_image().write_to(&mut bytes, format)?;
        Ok(bytes.into_inner())
    }

    fn decoded_png(png: &[u8]) -> anyhow::Result<DynamicImage> {
        Ok(image::load_from_memory_with_format(png, ImageFormat::Png)?)
    }

    #[test]
    fn image_files_of_any_format_become_png() -> anyhow::Result<()> {
        let tiff = encoded(ImageOutputFormat::Tiff)?;
        let png = ClipboardImage::new(ClipboardImageEncoding::File, tiff).to_png()?;

        assert_eq!(decoded_png(&png)?.to_rgba8(), sample_image().to_rgba8());
        Ok(())
    }

    #[test]
    fn windows_bitmaps_become_png() -> anyhow::Result<()> {
        let bmp = encoded(ImageOutputFormat::Bmp)?;
        let bitmap = bmp[BITMAP_FILE_HEADER_SIZE as usize..].to_vec();
        let png = ClipboardImage::new(ClipboardImageEncoding::WindowsBitmap, bitmap).to_png()?;

        assert_eq!(decoded_png(&png)?.to_rgba8(), sample_image().to_rgba8());
        Ok(())
    }

    #[test]
    fn the_rebuilt_file_header_points_at_the_pixels() -> anyhow::Result<()> {
        let bmp = encoded(ImageOutputFormat::Bmp)?;
        let rebuilt = bitmap_file_of(&bmp[BITMAP_FILE_HEADER_SIZE as usize..]);

        assert_eq!(rebuilt.as_deref(), Some(bmp.as_slice()));
        Ok(())
    }

    #[test]
    fn broken_images_are_refused() {
        assert!(matches!(
            ClipboardImage::new(ClipboardImageEncoding::File, b"not an image".to_vec()).to_png(),
            Err(ClipboardError::UnreadableImage(_))
        ));
        assert!(matches!(
            ClipboardImage::new(ClipboardImageEncoding::WindowsBitmap, vec![1, 2]).to_png(),
            Err(ClipboardError::DamagedBitmapHeader)
        ));
    }
}
