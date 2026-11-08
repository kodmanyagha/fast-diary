use std::{
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use chrono::NaiveDateTime;

use super::{
    asset_name::{
        asset_directory_name, asset_reference, image_asset_file_name, numbered_file_name,
        relative_asset_path, ASSETS_DIRECTORY,
    },
    codec::Codec,
    error::{StorageError, StorageResult},
};

const MAX_NAME_ATTEMPTS: usize = 1000;

/// The largest image, in bytes, that is copied into a diary folder.
pub const MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024;

/// Tells whether `content` is an image whose size can be read, which damaged files and files
/// that merely carry an image extension are not.
fn is_readable_image(content: &[u8]) -> bool {
    image::io::Reader::new(Cursor::new(content))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .is_some()
}

/// Keeps the files that diaries refer to, such as images, under the `assets` directory of a
/// diary folder. Assets of an encrypted folder are encrypted with the key of the folder.
#[derive(Debug, Clone)]
pub struct AssetStore {
    base_path: PathBuf,
    codec: Codec,
}

impl AssetStore {
    pub fn new(base_path: impl Into<PathBuf>, codec: Codec) -> Self {
        Self {
            base_path: base_path.into(),
            codec,
        }
    }

    /// Copies the image at `source` into `assets/<diary date and time>/` of the diary
    /// `diary_file_name` under a slug of its original name followed by `copied_at`, and returns
    /// the reference that the diary uses for it.
    pub fn import_image(
        &self,
        diary_file_name: &str,
        source: &Path,
        copied_at: NaiveDateTime,
    ) -> StorageResult<String> {
        let size = fs::metadata(source)
            .map_err(StorageError::io_at(source))?
            .len();
        if size > MAX_IMAGE_BYTES {
            return Err(StorageError::ImageTooLarge {
                path: source.to_path_buf(),
                limit: MAX_IMAGE_BYTES,
            });
        }
        let content = fs::read(source).map_err(StorageError::io_at(source))?;

        self.import_image_data(diary_file_name, source, &content, copied_at)
    }

    /// Stores the image `content` like [`AssetStore::import_image`] does, naming it after
    /// `original_name`, such as `screenshot.png` for an image pasted from the clipboard.
    pub fn import_image_data(
        &self,
        diary_file_name: &str,
        original_name: &Path,
        content: &[u8],
        copied_at: NaiveDateTime,
    ) -> StorageResult<String> {
        let directory_name = asset_directory_name(diary_file_name)
            .ok_or_else(|| StorageError::InvalidFileName(diary_file_name.to_string()))?;
        let file_name = image_asset_file_name(original_name, copied_at)
            .ok_or_else(|| StorageError::UnsupportedImage(original_name.to_path_buf()))?;
        let is_too_large = u64::try_from(content.len()).map_or(true, |size| size > MAX_IMAGE_BYTES);
        if is_too_large {
            return Err(StorageError::ImageTooLarge {
                path: original_name.to_path_buf(),
                limit: MAX_IMAGE_BYTES,
            });
        }
        if !is_readable_image(content) {
            return Err(StorageError::UnsupportedImage(original_name.to_path_buf()));
        }

        let directory = self.base_path.join(ASSETS_DIRECTORY).join(directory_name);
        fs::create_dir_all(&directory).map_err(StorageError::io_at(&directory))?;
        let stored_name = self.store_under_free_name(&directory, &file_name, content)?;

        Ok(asset_reference(directory_name, &stored_name))
    }

    /// Reads the asset that `reference` points at and returns its original content.
    pub fn read(&self, reference: &str) -> StorageResult<Vec<u8>> {
        let relative_path = relative_asset_path(reference)
            .ok_or_else(|| StorageError::InvalidFileName(reference.to_string()))?;
        let disk_path = self.disk_path_of(&self.base_path.join(relative_path))?;

        fs::read(&disk_path)
            .map_err(StorageError::io_at(&disk_path))
            .and_then(|stored| self.codec.decode_bytes(&disk_path, &stored))
    }

    /// Returns where the asset at `path` is kept on disk in the codec of this store.
    fn disk_path_of(&self, path: &Path) -> StorageResult<PathBuf> {
        path.file_name()
            .map(|file_name| {
                path.with_file_name(self.codec.asset_disk_name(&file_name.to_string_lossy()))
            })
            .ok_or_else(|| StorageError::InvalidFileName(path.display().to_string()))
    }

    /// Writes `content` into `directory` under `file_name`, or under a numbered variant of it
    /// when that name is taken, never replacing an existing file. Returns the name it got.
    fn store_under_free_name(
        &self,
        directory: &Path,
        file_name: &str,
        content: &[u8],
    ) -> StorageResult<String> {
        (1..=MAX_NAME_ATTEMPTS)
            .map(|number| numbered_file_name(file_name, number))
            .find_map(
                |candidate| match self.write_new(&directory.join(&candidate), content) {
                    Ok(()) => Some(Ok(candidate)),
                    Err(StorageError::Io { source, .. })
                        if source.kind() == std::io::ErrorKind::AlreadyExists =>
                    {
                        None
                    }
                    Err(err) => Some(Err(err)),
                },
            )
            .unwrap_or_else(|| Err(StorageError::InvalidFileName(file_name.to_string())))
    }

    /// Creates the file of the asset at `path` with `content`, failing when it already exists.
    fn write_new(&self, path: &Path, content: &[u8]) -> StorageResult<()> {
        let disk_path = self.disk_path_of(path)?;
        let stored = self.codec.encode_bytes(&disk_path, content)?;

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&disk_path)
            .and_then(|mut file| file.write_all(&stored).and_then(|()| file.sync_all()))
            .map_err(StorageError::io_at(&disk_path))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::NaiveDate;

    use super::*;
    use crate::vault::folder_key::FolderKey;

    const IMAGE_BYTES: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fn moment() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 8)
            .and_then(|date| date.and_hms_opt(17, 14, 5))
            .unwrap_or_default()
    }

    fn source_image(dir: &Path, name: &str) -> anyhow::Result<PathBuf> {
        let path = dir.join(name);
        fs::write(&path, IMAGE_BYTES)?;
        Ok(path)
    }

    #[test]
    fn an_image_is_copied_under_the_directory_of_its_diary() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let source = source_image(outside.path(), "Deniz Kenarı.png")?;
        let store = AssetStore::new(folder.path(), Codec::Plain);

        let reference = store.import_image("251108171400.md", &source, moment())?;

        assert_eq!(
            reference,
            "assets/251108171400/deniz_kenari_20261108171405.png"
        );
        assert_eq!(fs::read(folder.path().join(&reference))?, IMAGE_BYTES);
        assert_eq!(store.read(&reference)?, IMAGE_BYTES);
        assert!(source.exists());
        Ok(())
    }

    #[test]
    fn a_taken_name_gets_a_number_instead_of_being_replaced() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let source = source_image(outside.path(), "a.png")?;
        let store = AssetStore::new(folder.path(), Codec::Plain);

        let first = store.import_image("251108171400.md", &source, moment())?;
        let second = store.import_image("251108171400.md", &source, moment())?;

        assert_eq!(first, "assets/251108171400/a_20261108171405.png");
        assert_eq!(second, "assets/251108171400/a_20261108171405_2.png");
        Ok(())
    }

    #[test]
    fn an_encrypted_folder_keeps_its_images_encrypted() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let source = source_image(outside.path(), "secret.png")?;
        let store = AssetStore::new(
            folder.path(),
            Codec::Encrypted(Arc::new(FolderKey::generate()?)),
        );

        let reference = store.import_image("251108171400.md.enc", &source, moment())?;

        assert!(!folder.path().join(&reference).exists());
        let stored = fs::read(folder.path().join(format!("{reference}.enc")))?;
        assert!(stored.starts_with(b"FDRY"));
        assert!(!stored.windows(3).any(|window| window == b"PNG"));
        assert_eq!(store.read(&reference)?, IMAGE_BYTES);
        Ok(())
    }

    #[test]
    fn files_that_are_not_images_are_not_copied() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let source = source_image(outside.path(), "notes.txt")?;
        let store = AssetStore::new(folder.path(), Codec::Plain);

        assert!(matches!(
            store.import_image("251108171400.md", &source, moment()),
            Err(StorageError::UnsupportedImage(_))
        ));
        assert!(!folder.path().join(ASSETS_DIRECTORY).exists());
        Ok(())
    }

    #[test]
    fn pasted_image_data_is_named_after_the_given_name() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let store = AssetStore::new(folder.path(), Codec::Plain);

        let reference = store.import_image_data(
            "251108171400.md",
            Path::new("screenshot.png"),
            IMAGE_BYTES,
            moment(),
        )?;

        assert_eq!(
            reference,
            "assets/251108171400/screenshot_20261108171405.png"
        );
        assert_eq!(store.read(&reference)?, IMAGE_BYTES);
        Ok(())
    }

    #[test]
    fn files_that_only_look_like_images_are_not_copied() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let fake = outside.path().join("fake.png");
        fs::write(&fake, b"definitely not a png")?;
        let store = AssetStore::new(folder.path(), Codec::Plain);

        assert!(matches!(
            store.import_image("251108171400.md", &fake, moment()),
            Err(StorageError::UnsupportedImage(_))
        ));
        assert!(!folder.path().join(ASSETS_DIRECTORY).exists());
        Ok(())
    }

    #[test]
    fn images_over_the_size_limit_are_not_copied() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let huge = outside.path().join("huge.png");
        fs::File::create(&huge)?.set_len(MAX_IMAGE_BYTES + 1)?;
        let store = AssetStore::new(folder.path(), Codec::Plain);

        assert!(matches!(
            store.import_image("251108171400.md", &huge, moment()),
            Err(StorageError::ImageTooLarge { .. })
        ));
        Ok(())
    }

    #[test]
    fn references_outside_the_folder_are_never_read() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let store = AssetStore::new(folder.path().join("inner"), Codec::Plain);
        fs::write(folder.path().join("outside.png"), IMAGE_BYTES)?;

        assert!(matches!(
            store.read("../outside.png"),
            Err(StorageError::InvalidFileName(_))
        ));
        Ok(())
    }
}
