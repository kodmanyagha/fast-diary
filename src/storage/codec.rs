use std::{path::Path, sync::Arc};

use crate::vault::{
    cipher::{decrypt_file, encrypt_file},
    folder_key::FolderKey,
};

use super::error::{StorageError, StorageResult};

const PLAIN_EXTENSION: &str = "md";
const ENCRYPTED_EXTENSION: &str = "md.enc";
const ENCRYPTED_SUFFIX: &str = ".enc";

#[derive(Debug, Clone)]
pub enum Codec {
    Plain,
    Encrypted(Arc<FolderKey>),
}

impl Codec {
    /// Returns the extension that new diaries get in this codec.
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Plain => PLAIN_EXTENSION,
            Self::Encrypted(_) => ENCRYPTED_EXTENSION,
        }
    }

    /// Tells whether `file_name` names a diary that this codec can read.
    pub fn accepts_file_name(&self, file_name: &str) -> bool {
        let is_visible = !file_name.starts_with('.');

        is_visible
            && match self {
                Self::Plain => !file_name.ends_with(".enc"),
                Self::Encrypted(_) => file_name.ends_with(".md.enc"),
            }
    }

    /// Returns the name that the asset `file_name` has on disk in this codec. Encrypted assets
    /// carry an extra `.enc`, so that they are never mistaken for readable files.
    pub fn asset_disk_name(&self, file_name: &str) -> String {
        match self {
            Self::Plain => file_name.to_string(),
            Self::Encrypted(_) => format!("{file_name}{ENCRYPTED_SUFFIX}"),
        }
    }

    /// Converts diary text into the bytes stored on disk at `path`.
    pub fn encode(&self, path: &Path, text: &str) -> StorageResult<Vec<u8>> {
        self.encode_bytes(path, text.as_bytes())
    }

    /// Converts the bytes stored on disk at `path` back into diary text.
    pub fn decode(&self, path: &Path, content: &[u8]) -> StorageResult<String> {
        String::from_utf8(self.decode_bytes(path, content)?).map_err(|_| {
            StorageError::InvalidUtf8 {
                path: path.to_path_buf(),
            }
        })
    }

    /// Converts any content into the bytes stored on disk at `path`.
    pub fn encode_bytes(&self, path: &Path, content: &[u8]) -> StorageResult<Vec<u8>> {
        match self {
            Self::Plain => Ok(content.to_vec()),
            Self::Encrypted(key) => {
                encrypt_file(key, &file_name_of(path), content).map_err(|source| {
                    StorageError::Cipher {
                        path: path.to_path_buf(),
                        source,
                    }
                })
            }
        }
    }

    /// Converts the bytes stored on disk at `path` back into the content they were made from.
    pub fn decode_bytes(&self, path: &Path, stored: &[u8]) -> StorageResult<Vec<u8>> {
        match self {
            Self::Plain => Ok(stored.to_vec()),
            Self::Encrypted(key) => {
                decrypt_file(key, &file_name_of(path), stored).map_err(|source| {
                    StorageError::Cipher {
                        path: path.to_path_buf(),
                        source,
                    }
                })
            }
        }
    }
}

/// Returns the last component of `path` as text.
fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encrypted_codec() -> anyhow::Result<Codec> {
        Ok(Codec::Encrypted(Arc::new(FolderKey::generate()?)))
    }

    #[test]
    fn plain_codec_is_the_identity() -> anyhow::Result<()> {
        let path = Path::new("/d/240101.md");

        let bytes = Codec::Plain.encode(path, "merhaba")?;

        assert_eq!(bytes, "merhaba".as_bytes());
        assert_eq!(Codec::Plain.decode(path, &bytes)?, "merhaba");
        Ok(())
    }

    #[test]
    fn encrypted_codec_round_trips_and_hides_text() -> anyhow::Result<()> {
        let codec = encrypted_codec()?;
        let path = Path::new("/d/240101.md.enc");

        let bytes = codec.encode(path, "merhaba dünya")?;

        assert!(!bytes.windows(7).any(|window| window == b"merhaba"));
        assert_eq!(codec.decode(path, &bytes)?, "merhaba dünya");
        Ok(())
    }

    #[test]
    fn encrypted_codec_rejects_plain_bytes() -> anyhow::Result<()> {
        assert!(matches!(
            encrypted_codec()?.decode(Path::new("/d/a.md.enc"), b"plain text"),
            Err(StorageError::Cipher { .. })
        ));
        Ok(())
    }

    #[test]
    fn bytes_that_are_not_text_round_trip_in_both_codecs() -> anyhow::Result<()> {
        let path = Path::new("/d/assets/2511/a.png.enc");
        let binary = [0x89, b'P', b'N', b'G', 0xff, 0x00, 0xfe];

        [Codec::Plain, encrypted_codec()?]
            .iter()
            .try_for_each(|codec| {
                let stored = codec.encode_bytes(path, &binary)?;
                anyhow::ensure!(codec.decode_bytes(path, &stored)? == binary);
                Ok(())
            })
    }

    #[test]
    fn encrypted_assets_get_an_extra_extension_on_disk() -> anyhow::Result<()> {
        assert_eq!(Codec::Plain.asset_disk_name("a.png"), "a.png");
        assert_eq!(encrypted_codec()?.asset_disk_name("a.png"), "a.png.enc");
        Ok(())
    }

    #[test]
    fn extensions_and_file_filters_match_the_codec() -> anyhow::Result<()> {
        let encrypted = encrypted_codec()?;

        assert_eq!(Codec::Plain.file_extension(), "md");
        assert_eq!(encrypted.file_extension(), "md.enc");
        assert!(Codec::Plain.accepts_file_name("240101.md"));
        assert!(!Codec::Plain.accepts_file_name("240101.md.enc"));
        assert!(!Codec::Plain.accepts_file_name(".240101.md.tmp"));
        assert!(encrypted.accepts_file_name("240101.md.enc"));
        assert!(!encrypted.accepts_file_name("240101.md"));
        assert!(!encrypted.accepts_file_name(".encrypted"));
        Ok(())
    }
}
