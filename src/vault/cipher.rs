use super::{
    aead::{self, NONCE_LENGTH},
    error::{VaultError, VaultResult},
    folder_key::FolderKey,
    random_array,
};

const MAGIC: &[u8; 4] = b"FDRY";
const FORMAT_VERSION: u8 = 1;

/// Encrypts a diary into the binary format `FDRY | version | nonce | ciphertext+tag`.
/// The file name is authenticated, so an encrypted diary cannot be swapped with another.
pub fn encrypt_file(key: &FolderKey, file_name: &str, plaintext: &[u8]) -> VaultResult<Vec<u8>> {
    let nonce = random_array::<NONCE_LENGTH>()?;
    let ciphertext = aead::seal(
        key.as_bytes(),
        &nonce,
        &associated_data(file_name),
        plaintext,
    )?;

    Ok([MAGIC.as_slice(), &[FORMAT_VERSION], &nonce, &ciphertext].concat())
}

/// Decrypts a diary produced by [`encrypt_file`] for the same `file_name`.
pub fn decrypt_file(key: &FolderKey, file_name: &str, content: &[u8]) -> VaultResult<Vec<u8>> {
    let body = content
        .strip_prefix(MAGIC.as_slice())
        .and_then(<[u8]>::split_first)
        .filter(|(version, _)| **version == FORMAT_VERSION)
        .map(|(_, body)| body)
        .filter(|body| body.len() >= NONCE_LENGTH)
        .ok_or(VaultError::CorruptedContent)?;
    let (nonce, ciphertext) = body.split_at(NONCE_LENGTH);

    aead::open(
        key.as_bytes(),
        nonce,
        &associated_data(file_name),
        ciphertext,
    )
    .ok_or(VaultError::CorruptedContent)
}

/// Builds the authenticated header that binds the format version and file name.
fn associated_data(file_name: &str) -> Vec<u8> {
    [MAGIC.as_slice(), &[FORMAT_VERSION], file_name.as_bytes()].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decrypts_what_was_encrypted() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;

        let encrypted = encrypt_file(&key, "240101.md.enc", "günaydın".as_bytes())?;

        assert_eq!(
            decrypt_file(&key, "240101.md.enc", &encrypted)?,
            "günaydın".as_bytes()
        );
        Ok(())
    }

    #[test]
    fn output_is_binary_and_hides_the_text() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;

        let encrypted = encrypt_file(&key, "240101.md.enc", b"very secret sentence")?;

        assert!(encrypted.starts_with(b"FDRY"));
        assert!(!encrypted
            .windows(b"secret".len())
            .any(|window| window == b"secret"));
        Ok(())
    }

    #[test]
    fn same_text_encrypts_differently_each_time() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;

        assert_ne!(
            encrypt_file(&key, "a.md.enc", b"same")?,
            encrypt_file(&key, "a.md.enc", b"same")?
        );
        Ok(())
    }

    #[test]
    fn empty_text_is_supported() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;

        let encrypted = encrypt_file(&key, "a.md.enc", b"")?;

        assert_eq!(decrypt_file(&key, "a.md.enc", &encrypted)?, b"");
        Ok(())
    }

    #[test]
    fn rejects_another_key() -> anyhow::Result<()> {
        let encrypted = encrypt_file(&FolderKey::generate()?, "a.md.enc", b"text")?;

        assert!(matches!(
            decrypt_file(&FolderKey::generate()?, "a.md.enc", &encrypted),
            Err(VaultError::CorruptedContent)
        ));
        Ok(())
    }

    #[test]
    fn rejects_content_moved_to_another_file_name() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;
        let encrypted = encrypt_file(&key, "a.md.enc", b"text")?;

        assert!(decrypt_file(&key, "b.md.enc", &encrypted).is_err());
        Ok(())
    }

    #[test]
    fn rejects_truncated_tampered_and_foreign_content() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;
        let encrypted = encrypt_file(&key, "a.md.enc", b"text")?;
        let tampered = [
            &encrypted[..encrypted.len() - 1],
            &[encrypted[encrypted.len() - 1] ^ 1],
        ]
        .concat();

        assert!(decrypt_file(&key, "a.md.enc", &encrypted[..10]).is_err());
        assert!(decrypt_file(&key, "a.md.enc", &tampered).is_err());
        assert!(decrypt_file(&key, "a.md.enc", b"plain old markdown text, not encrypted").is_err());
        assert!(decrypt_file(&key, "a.md.enc", b"").is_err());
        Ok(())
    }
}
