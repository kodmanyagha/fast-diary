use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use super::{
    aead::{self, NONCE_LENGTH},
    error::{VaultError, VaultResult},
    folder_key::FolderKey,
    kdf::{derive_key, KdfParams, KDF_ALGORITHM_NAME},
    random_array,
};

const KEY_FILE_VERSION: u32 = 1;
const CIPHER_NAME: &str = "xchacha20poly1305";
const SALT_LENGTH: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFile {
    version: u32,
    kdf: KdfSection,
    cipher: String,
    nonce: String,
    ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KdfSection {
    algorithm: String,
    #[serde(flatten)]
    params: KdfParams,
    salt: String,
}

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
struct KeyPayload {
    key: String,
}

impl KeyFile {
    /// Encrypts `folder_key` under a key derived from `password`, using a fresh salt and nonce.
    pub fn seal(password: &str, folder_key: &FolderKey, params: &KdfParams) -> VaultResult<Self> {
        let salt = random_array::<SALT_LENGTH>()?;
        let nonce = random_array::<NONCE_LENGTH>()?;
        let kdf = KdfSection {
            algorithm: KDF_ALGORITHM_NAME.to_string(),
            params: *params,
            salt: STANDARD.encode(salt),
        };

        let payload = Zeroizing::new(
            serde_json::to_vec(&KeyPayload {
                key: folder_key.to_base64().to_string(),
            })
            .map_err(|err| VaultError::Encryption(err.to_string()))?,
        );
        let wrapping_key = derive_key(password, &salt, params)?;
        let ciphertext = aead::seal(
            &wrapping_key[..],
            &nonce,
            &associated_data(KEY_FILE_VERSION, &kdf),
            &payload,
        )?;

        Ok(Self {
            version: KEY_FILE_VERSION,
            kdf,
            cipher: CIPHER_NAME.to_string(),
            nonce: STANDARD.encode(nonce),
            ciphertext: STANDARD.encode(ciphertext),
        })
    }

    /// Recovers the folder key with `password`, failing with `WrongPassword` when the
    /// password does not authenticate the file.
    pub fn open(&self, password: &str) -> VaultResult<FolderKey> {
        self.ensure_supported()?;

        let salt = decode_field("salt", &self.kdf.salt)?;
        let nonce = decode_field("nonce", &self.nonce)?;
        let ciphertext = decode_field("ciphertext", &self.ciphertext)?;
        (nonce.len() == NONCE_LENGTH).then_some(()).ok_or_else(|| {
            VaultError::InvalidKeyFile("The nonce has a wrong length.".to_string())
        })?;

        let wrapping_key = derive_key(password, &salt, &self.kdf.params)?;
        let payload = aead::open(
            &wrapping_key[..],
            &nonce,
            &associated_data(self.version, &self.kdf),
            &ciphertext,
        )
        .map(Zeroizing::new)
        .ok_or(VaultError::WrongPassword)?;

        let payload: KeyPayload = serde_json::from_slice(&payload)
            .map_err(|err| VaultError::InvalidKeyFile(err.to_string()))?;
        FolderKey::from_base64(&payload.key)
    }

    /// Serializes the key file as pretty printed JSON.
    pub fn to_json_bytes(&self) -> VaultResult<Vec<u8>> {
        serde_json::to_vec_pretty(self).map_err(|err| VaultError::InvalidKeyFile(err.to_string()))
    }

    /// Parses a key file from its JSON form.
    pub fn from_json_bytes(bytes: &[u8]) -> VaultResult<Self> {
        serde_json::from_slice(bytes).map_err(|err| VaultError::InvalidKeyFile(err.to_string()))
    }

    /// Rejects key files written by an unknown format version or with unknown algorithms.
    fn ensure_supported(&self) -> VaultResult<()> {
        (self.version == KEY_FILE_VERSION
            && self.cipher == CIPHER_NAME
            && self.kdf.algorithm == KDF_ALGORITHM_NAME)
            .then_some(())
            .ok_or_else(|| {
                VaultError::InvalidKeyFile(format!(
                    "Unsupported format (version {}, {}, {}).",
                    self.version, self.cipher, self.kdf.algorithm
                ))
            })
    }
}

/// Decodes a base64 field of the key file, naming the field on failure.
fn decode_field(field_name: &str, encoded: &str) -> VaultResult<Vec<u8>> {
    STANDARD
        .decode(encoded)
        .map_err(|_| VaultError::InvalidKeyFile(format!("The {field_name} is not valid base64.")))
}

/// Builds the authenticated header, so tampering with the KDF settings or salt is detected.
fn associated_data(version: u32, kdf: &KdfSection) -> Vec<u8> {
    format!(
        "fast-diary/key-file/v{version}/{}/m={}/t={}/p={}/salt={}",
        kdf.algorithm,
        kdf.params.memory_kib,
        kdf.params.iterations,
        kdf.params.parallelism,
        kdf.salt
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::kdf::TEST_PARAMS;

    #[test]
    fn opens_with_the_password_it_was_sealed_with() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;
        let key_file = KeyFile::seal("correct horse", &key, &TEST_PARAMS)?;

        assert_eq!(key_file.open("correct horse")?.as_bytes(), key.as_bytes());
        Ok(())
    }

    #[test]
    fn wrong_password_is_reported_as_such() -> anyhow::Result<()> {
        let key_file = KeyFile::seal("correct horse", &FolderKey::generate()?, &TEST_PARAMS)?;

        assert!(matches!(
            key_file.open("battery staple"),
            Err(VaultError::WrongPassword)
        ));
        Ok(())
    }

    #[test]
    fn json_round_trip_keeps_the_file_openable() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;
        let bytes = KeyFile::seal("correct horse", &key, &TEST_PARAMS)?.to_json_bytes()?;

        let restored = KeyFile::from_json_bytes(&bytes)?;

        assert_eq!(restored.open("correct horse")?.as_bytes(), key.as_bytes());
        Ok(())
    }

    #[test]
    fn serialized_file_does_not_contain_the_key() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;
        let json = String::from_utf8(
            KeyFile::seal("correct horse", &key, &TEST_PARAMS)?.to_json_bytes()?,
        )?;

        assert!(!json.contains(key.to_base64().as_str()));
        Ok(())
    }

    #[test]
    fn sealing_twice_uses_fresh_salt_and_nonce() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;
        let first = KeyFile::seal("correct horse", &key, &TEST_PARAMS)?;
        let second = KeyFile::seal("correct horse", &key, &TEST_PARAMS)?;

        assert_ne!(first.kdf.salt, second.kdf.salt);
        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.ciphertext, second.ciphertext);
        Ok(())
    }

    #[test]
    fn tampering_with_kdf_settings_is_detected() -> anyhow::Result<()> {
        let mut key_file = KeyFile::seal("correct horse", &FolderKey::generate()?, &TEST_PARAMS)?;
        key_file.kdf.params.iterations += 1;

        assert!(matches!(
            key_file.open("correct horse"),
            Err(VaultError::WrongPassword)
        ));
        Ok(())
    }

    #[test]
    fn rejects_unknown_versions_and_garbage() -> anyhow::Result<()> {
        let mut key_file = KeyFile::seal("correct horse", &FolderKey::generate()?, &TEST_PARAMS)?;
        key_file.version = 99;

        assert!(matches!(
            key_file.open("correct horse"),
            Err(VaultError::InvalidKeyFile(_))
        ));
        assert!(KeyFile::from_json_bytes(b"not json").is_err());
        Ok(())
    }
}
