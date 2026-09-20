pub mod aead;
pub mod cipher;
pub mod error;
pub mod folder;
pub mod folder_key;
pub mod kdf;
pub mod key_file;
pub mod migration;
pub mod password;

use error::{VaultError, VaultResult};

/// Returns `N` bytes from the operating system's secure random generator.
pub fn random_array<const N: usize>() -> VaultResult<[u8; N]> {
    let mut bytes = [0_u8; N];
    getrandom::fill(&mut bytes).map_err(|err| VaultError::Randomness(err.to_string()))?;
    Ok(bytes)
}
