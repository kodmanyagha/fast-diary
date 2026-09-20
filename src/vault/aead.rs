use chacha20poly1305::{
    aead::{Aead, Payload},
    KeyInit, XChaCha20Poly1305, XNonce,
};

use super::error::{VaultError, VaultResult};

pub const NONCE_LENGTH: usize = 24;

/// Encrypts `plaintext` with XChaCha20-Poly1305, binding `associated_data` to the result.
pub fn seal(
    key: &[u8],
    nonce: &[u8; NONCE_LENGTH],
    associated_data: &[u8],
    plaintext: &[u8],
) -> VaultResult<Vec<u8>> {
    XChaCha20Poly1305::new_from_slice(key)
        .map_err(|err| VaultError::Encryption(err.to_string()))?
        .encrypt(
            &XNonce::from(*nonce),
            Payload {
                msg: plaintext,
                aad: associated_data,
            },
        )
        .map_err(|err| VaultError::Encryption(err.to_string()))
}

/// Decrypts `ciphertext`, returning `None` when the key, nonce, associated data or
/// ciphertext do not authenticate.
pub fn open(
    key: &[u8],
    nonce: &[u8],
    associated_data: &[u8],
    ciphertext: &[u8],
) -> Option<Vec<u8>> {
    let nonce = XNonce::try_from(nonce).ok()?;

    XChaCha20Poly1305::new_from_slice(key)
        .ok()?
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: associated_data,
            },
        )
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [7; 32];
    const NONCE: [u8; NONCE_LENGTH] = [9; NONCE_LENGTH];

    #[test]
    fn opens_what_was_sealed() -> anyhow::Result<()> {
        let sealed = seal(&KEY, &NONCE, b"aad", b"secret")?;

        assert_eq!(
            open(&KEY, &NONCE, b"aad", &sealed),
            Some(b"secret".to_vec())
        );
        Ok(())
    }

    #[test]
    fn rejects_wrong_key_nonce_or_associated_data() -> anyhow::Result<()> {
        let sealed = seal(&KEY, &NONCE, b"aad", b"secret")?;

        assert_eq!(open(&[8; 32], &NONCE, b"aad", &sealed), None);
        assert_eq!(open(&KEY, &[1; NONCE_LENGTH], b"aad", &sealed), None);
        assert_eq!(open(&KEY, &NONCE, b"other", &sealed), None);
        assert_eq!(open(&KEY, &NONCE[..10], b"aad", &sealed), None);
        Ok(())
    }

    #[test]
    fn rejects_tampered_ciphertext() -> anyhow::Result<()> {
        let mut sealed = seal(&KEY, &NONCE, b"aad", b"secret")?;
        sealed[0] ^= 1;

        assert_eq!(open(&KEY, &NONCE, b"aad", &sealed), None);
        Ok(())
    }
}
