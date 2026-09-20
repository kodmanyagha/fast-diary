use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::storage::atomic_write::write_atomically;

use super::{
    error::{VaultError, VaultResult},
    folder_key::FolderKey,
    kdf::KdfParams,
    key_file::KeyFile,
    password::validate_new_password,
};

pub const KEY_FILE_NAME: &str = ".encrypted";
const KEY_FILE_BACKUP_NAME: &str = ".encrypted.bak";

/// Returns the path of the `.encrypted` file of the diary folder `folder`.
pub fn key_file_path(folder: &Path) -> PathBuf {
    folder.join(KEY_FILE_NAME)
}

/// Tells whether `folder` holds encrypted diaries, which is the case when it has a `.encrypted` file.
pub fn is_encrypted_folder(folder: &Path) -> bool {
    key_file_path(folder).is_file()
}

/// Turns `folder` into an encrypted folder protected by `password`: generates a random folder
/// key and stores it, encrypted with the password, in a new `.encrypted` file.
pub fn create_encrypted_folder(
    folder: &Path,
    password: &str,
    confirmation: &str,
    params: &KdfParams,
) -> VaultResult<FolderKey> {
    if is_encrypted_folder(folder) {
        return Err(VaultError::AlreadyEncrypted);
    }
    validate_new_password(password, confirmation)?;

    let folder_key = FolderKey::generate()?;
    write_key_file(folder, &KeyFile::seal(password, &folder_key, params)?)?;

    Ok(folder_key)
}

/// Recovers the folder key of `folder` using `password`.
pub fn unlock_folder(folder: &Path, password: &str) -> VaultResult<FolderKey> {
    read_key_file(folder)?.open(password)
}

/// Re-protects the folder key with a new password. Only the `.encrypted` file changes, the
/// encrypted diaries stay untouched. The previous `.encrypted` file is kept as a backup.
pub fn change_password(
    folder: &Path,
    current_password: &str,
    new_password: &str,
    confirmation: &str,
    params: &KdfParams,
) -> VaultResult<()> {
    let current_bytes =
        fs::read(key_file_path(folder)).map_err(VaultError::io_at(&key_file_path(folder)))?;
    let folder_key = KeyFile::from_json_bytes(&current_bytes)?.open(current_password)?;
    validate_new_password(new_password, confirmation)?;

    write_atomically(&folder.join(KEY_FILE_BACKUP_NAME), &current_bytes)?;
    write_key_file(folder, &KeyFile::seal(new_password, &folder_key, params)?)
}

/// Reads and parses the `.encrypted` file of `folder`.
fn read_key_file(folder: &Path) -> VaultResult<KeyFile> {
    let path = key_file_path(folder);

    fs::read(&path)
        .map_err(VaultError::io_at(&path))
        .and_then(|bytes| KeyFile::from_json_bytes(&bytes))
}

/// Atomically replaces the `.encrypted` file of `folder`.
fn write_key_file(folder: &Path, key_file: &KeyFile) -> VaultResult<()> {
    write_atomically(&key_file_path(folder), &key_file.to_json_bytes()?).map_err(VaultError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::kdf::TEST_PARAMS;

    #[test]
    fn plain_folder_is_not_reported_as_encrypted() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;

        assert!(!is_encrypted_folder(dir.path()));
        Ok(())
    }

    #[test]
    fn created_folder_unlocks_with_its_password_only() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;

        let key = create_encrypted_folder(dir.path(), "123456", "123456", &TEST_PARAMS)?;

        assert!(is_encrypted_folder(dir.path()));
        assert_eq!(
            unlock_folder(dir.path(), "123456")?.as_bytes(),
            key.as_bytes()
        );
        assert!(matches!(
            unlock_folder(dir.path(), "654321"),
            Err(VaultError::WrongPassword)
        ));
        Ok(())
    }

    #[test]
    fn key_file_is_json_without_plain_key() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = create_encrypted_folder(dir.path(), "123456", "123456", &TEST_PARAMS)?;

        let content = fs::read_to_string(key_file_path(dir.path()))?;

        assert!(serde_json::from_str::<serde_json::Value>(&content).is_ok());
        assert!(!content.contains(key.to_base64().as_str()));
        Ok(())
    }

    #[test]
    fn refuses_weak_mismatching_or_repeated_creation() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;

        assert!(matches!(
            create_encrypted_folder(dir.path(), "12345", "12345", &TEST_PARAMS),
            Err(VaultError::PasswordTooShort { .. })
        ));
        assert!(matches!(
            create_encrypted_folder(dir.path(), "123456", "123457", &TEST_PARAMS),
            Err(VaultError::PasswordMismatch)
        ));
        assert!(!is_encrypted_folder(dir.path()));

        create_encrypted_folder(dir.path(), "123456", "123456", &TEST_PARAMS)?;
        assert!(matches!(
            create_encrypted_folder(dir.path(), "abcdef", "abcdef", &TEST_PARAMS),
            Err(VaultError::AlreadyEncrypted)
        ));
        Ok(())
    }

    #[test]
    fn changing_password_keeps_the_same_folder_key() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = create_encrypted_folder(dir.path(), "123456", "123456", &TEST_PARAMS)?;

        change_password(
            dir.path(),
            "123456",
            "new-secret",
            "new-secret",
            &TEST_PARAMS,
        )?;

        assert_eq!(
            unlock_folder(dir.path(), "new-secret")?.as_bytes(),
            key.as_bytes()
        );
        assert!(unlock_folder(dir.path(), "123456").is_err());
        Ok(())
    }

    #[test]
    fn changing_password_keeps_a_backup_of_the_previous_key_file() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        create_encrypted_folder(dir.path(), "123456", "123456", &TEST_PARAMS)?;
        let before = fs::read(key_file_path(dir.path()))?;

        change_password(
            dir.path(),
            "123456",
            "new-secret",
            "new-secret",
            &TEST_PARAMS,
        )?;

        assert_eq!(fs::read(dir.path().join(KEY_FILE_BACKUP_NAME))?, before);
        Ok(())
    }

    #[test]
    fn failed_password_change_leaves_key_file_untouched() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        create_encrypted_folder(dir.path(), "123456", "123456", &TEST_PARAMS)?;
        let before = fs::read(key_file_path(dir.path()))?;

        assert!(matches!(
            change_password(
                dir.path(),
                "wrong!",
                "new-secret",
                "new-secret",
                &TEST_PARAMS
            ),
            Err(VaultError::WrongPassword)
        ));
        assert!(matches!(
            change_password(dir.path(), "123456", "short", "short", &TEST_PARAMS),
            Err(VaultError::PasswordTooShort { .. })
        ));
        assert_eq!(fs::read(key_file_path(dir.path()))?, before);
        Ok(())
    }
}
