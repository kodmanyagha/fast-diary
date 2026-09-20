use std::{fs, path::Path};

use crate::storage::{
    atomic_write::write_atomically,
    history::{history_directory, snapshot_file_paths},
};

use super::{
    cipher::{decrypt_file, encrypt_file},
    error::{VaultError, VaultResult},
    folder_key::FolderKey,
};

const PLAIN_DIARY_EXTENSION: &str = ".md";
const ENCRYPTED_SUFFIX: &str = ".enc";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationReport {
    pub migrated: usize,
    pub failed: usize,
}

/// Encrypts every plain `.md` diary of `folder` (and its history) into `<name>.md.enc`.
///
/// Each encrypted copy is read back and verified before its plain original is deleted, so an
/// interrupted run loses nothing and can simply be repeated. Failures of single diaries are
/// logged and counted, they do not stop the remaining diaries from being migrated.
pub fn encrypt_plain_diaries(folder: &Path, key: &FolderKey) -> VaultResult<MigrationReport> {
    let outcomes = plain_diary_file_names(folder)?
        .iter()
        .map(|file_name| {
            migrate_diary(folder, key, file_name).inspect_err(|err| {
                tracing::warn!("Could not encrypt diary {file_name}: {err}");
            })
        })
        .collect::<Vec<_>>();

    let migrated = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
    Ok(MigrationReport {
        migrated,
        failed: outcomes.len() - migrated,
    })
}

/// Lists the visible plain `.md` files that sit directly inside `folder`.
fn plain_diary_file_names(folder: &Path) -> VaultResult<Vec<String>> {
    Ok(fs::read_dir(folder)
        .map_err(VaultError::io_at(folder))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|file_name| {
            file_name.ends_with(PLAIN_DIARY_EXTENSION) && !file_name.starts_with('.')
        })
        .collect())
}

/// Encrypts one diary with its history, then deletes the plain files.
fn migrate_diary(folder: &Path, key: &FolderKey, plain_name: &str) -> VaultResult<()> {
    let encrypted_name = format!("{plain_name}{ENCRYPTED_SUFFIX}");
    let plain_path = folder.join(plain_name);
    let encrypted_path = folder.join(&encrypted_name);
    let plaintext = fs::read(&plain_path).map_err(VaultError::io_at(&plain_path))?;

    let already_migrated = fs::read(&encrypted_path)
        .ok()
        .and_then(|content| decrypt_file(key, &encrypted_name, &content).ok())
        .is_some_and(|decrypted| decrypted == plaintext);

    if encrypted_path.exists() && !already_migrated {
        return Err(VaultError::MigrationConflict(encrypted_name));
    }
    if !already_migrated {
        write_verified(key, &encrypted_name, &encrypted_path, &plaintext)?;
    }

    migrate_history(folder, key, plain_name, &encrypted_name)?;
    fs::remove_file(&plain_path).map_err(VaultError::io_at(&plain_path))
}

/// Re-encrypts every snapshot of `plain_name` for `encrypted_name`, then removes the plain history.
fn migrate_history(
    folder: &Path,
    key: &FolderKey,
    plain_name: &str,
    encrypted_name: &str,
) -> VaultResult<()> {
    let plain_history = history_directory(folder, plain_name);
    if !plain_history.is_dir() {
        return Ok(());
    }

    let encrypted_history = history_directory(folder, encrypted_name);
    fs::create_dir_all(&encrypted_history).map_err(VaultError::io_at(&encrypted_history))?;

    snapshot_file_paths(&plain_history)?
        .iter()
        .try_for_each(|snapshot| {
            let plaintext = fs::read(snapshot).map_err(VaultError::io_at(snapshot))?;
            let target = encrypted_history.join(snapshot.file_name().unwrap_or_default());
            write_verified(key, encrypted_name, &target, &plaintext)
        })?;

    fs::remove_dir_all(&plain_history).map_err(VaultError::io_at(&plain_history))
}

/// Writes the encrypted form of `plaintext` to `path` and proves it decrypts back to it.
fn write_verified(
    key: &FolderKey,
    encrypted_name: &str,
    path: &Path,
    plaintext: &[u8],
) -> VaultResult<()> {
    write_atomically(path, &encrypt_file(key, encrypted_name, plaintext)?)?;

    let written = fs::read(path).map_err(VaultError::io_at(path))?;
    (decrypt_file(key, encrypted_name, &written)? == plaintext)
        .then_some(())
        .ok_or(VaultError::VerificationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_decrypted(folder: &Path, key: &FolderKey, file_name: &str) -> anyhow::Result<Vec<u8>> {
        Ok(decrypt_file(
            key,
            file_name,
            &fs::read(folder.join(file_name))?,
        )?)
    }

    #[test]
    fn encrypts_plain_diaries_and_removes_originals() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = FolderKey::generate()?;
        fs::write(dir.path().join("240101.md"), "first")?;
        fs::write(dir.path().join("240102.md"), "second")?;

        let report = encrypt_plain_diaries(dir.path(), &key)?;

        assert_eq!(
            report,
            MigrationReport {
                migrated: 2,
                failed: 0
            }
        );
        assert!(!dir.path().join("240101.md").exists());
        assert!(!dir.path().join("240102.md").exists());
        assert_eq!(read_decrypted(dir.path(), &key, "240101.md.enc")?, b"first");
        assert_eq!(
            read_decrypted(dir.path(), &key, "240102.md.enc")?,
            b"second"
        );
        Ok(())
    }

    #[test]
    fn leaves_other_files_alone() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = FolderKey::generate()?;
        fs::write(dir.path().join("notes.txt"), "notes")?;
        fs::write(dir.path().join(".hidden.md"), "hidden")?;

        let report = encrypt_plain_diaries(dir.path(), &key)?;

        assert_eq!(
            report,
            MigrationReport {
                migrated: 0,
                failed: 0
            }
        );
        assert!(dir.path().join("notes.txt").exists());
        assert!(dir.path().join(".hidden.md").exists());
        Ok(())
    }

    #[test]
    fn re_encrypts_history_and_removes_plain_snapshots() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = FolderKey::generate()?;
        fs::write(dir.path().join("240101.md"), "current")?;
        let plain_history = history_directory(dir.path(), "240101.md");
        fs::create_dir_all(&plain_history)?;
        fs::write(plain_history.join("0000000000001.snap"), "older")?;

        encrypt_plain_diaries(dir.path(), &key)?;

        assert!(!plain_history.exists());
        let encrypted_history = history_directory(dir.path(), "240101.md.enc");
        let snapshot = fs::read(encrypted_history.join("0000000000001.snap"))?;
        assert_eq!(decrypt_file(&key, "240101.md.enc", &snapshot)?, b"older");
        Ok(())
    }

    #[test]
    fn repeated_runs_finish_an_interrupted_migration() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = FolderKey::generate()?;
        fs::write(dir.path().join("240101.md"), "text")?;
        fs::write(
            dir.path().join("240101.md.enc"),
            encrypt_file(&key, "240101.md.enc", b"text")?,
        )?;

        let report = encrypt_plain_diaries(dir.path(), &key)?;

        assert_eq!(
            report,
            MigrationReport {
                migrated: 1,
                failed: 0
            }
        );
        assert!(!dir.path().join("240101.md").exists());
        Ok(())
    }

    #[test]
    fn keeps_plain_diary_when_encrypted_name_is_taken_by_different_content() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = FolderKey::generate()?;
        fs::write(dir.path().join("240101.md"), "plain text")?;
        fs::write(
            dir.path().join("240101.md.enc"),
            encrypt_file(&key, "240101.md.enc", b"different text")?,
        )?;

        let report = encrypt_plain_diaries(dir.path(), &key)?;

        assert_eq!(
            report,
            MigrationReport {
                migrated: 0,
                failed: 1
            }
        );
        assert_eq!(fs::read(dir.path().join("240101.md"))?, b"plain text");
        assert_eq!(
            read_decrypted(dir.path(), &key, "240101.md.enc")?,
            b"different text"
        );
        Ok(())
    }
}
