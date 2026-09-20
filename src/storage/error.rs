use std::{
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::vault::error::VaultError;

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error on {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{0:?} is not a plain file name.")]
    InvalidFileName(String),
    #[error("{path:?} does not contain valid UTF-8 text.")]
    InvalidUtf8 { path: PathBuf },
    #[error("Could not process {path:?}: {source}")]
    Cipher {
        path: PathBuf,
        #[source]
        source: VaultError,
    },
}

impl StorageError {
    /// Builds a closure that wraps an `io::Error` together with the path it occurred on.
    pub fn io_at(path: &Path) -> impl FnOnce(io::Error) -> Self + '_ {
        move |source| Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}
