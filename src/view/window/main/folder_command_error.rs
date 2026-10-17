use thiserror::Error;

use crate::vault::error::VaultError;

/// The failures of the folder commands that are not failures of the vault itself.
#[derive(Debug, Error)]
pub enum FolderCommandError {
    #[error("No folder is selected.")]
    NoFolderSelected,
    #[error("The folder is encrypted now, but its existing diaries could not be converted.")]
    DiariesNotConverted(#[source] VaultError),
    #[error("The folder is encrypted now, but {failed} existing diaries could not be converted.")]
    SomeDiariesNotConverted { failed: usize },
}
