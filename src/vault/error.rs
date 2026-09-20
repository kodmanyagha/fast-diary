use std::{io, path::PathBuf};

use thiserror::Error;

use crate::storage::error::StorageError;

pub type VaultResult<T> = Result<T, VaultError>;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Password must be at least {minimum} characters long.")]
    PasswordTooShort { minimum: usize },
    #[error("Passwords do not match.")]
    PasswordMismatch,
    #[error("Wrong password.")]
    WrongPassword,
    #[error("This folder is already encrypted.")]
    AlreadyEncrypted,
    #[error("The `.encrypted` file is damaged or unsupported: {0}")]
    InvalidKeyFile(String),
    #[error("The encrypted diary is damaged or was not encrypted with this folder's key.")]
    CorruptedContent,
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
    #[error("Encryption failed: {0}")]
    Encryption(String),
    #[error("Could not gather secure random bytes: {0}")]
    Randomness(String),
    #[error("`{0}` already exists with different content.")]
    MigrationConflict(String),
    #[error("Encrypted copy of a diary did not match its original.")]
    VerificationFailed,
    #[error("I/O error on {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Storage(#[from] Box<StorageError>),
}

impl VaultError {
    /// Builds a closure that wraps an `io::Error` together with the path it occurred on.
    pub fn io_at(path: &std::path::Path) -> impl FnOnce(io::Error) -> Self + '_ {
        move |source| Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

impl From<StorageError> for VaultError {
    fn from(error: StorageError) -> Self {
        Self::Storage(Box::new(error))
    }
}
