use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::storage::{
    asset_name::ASSETS_DIRECTORY,
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

/// Encrypts every plain `.md` diary of `folder` (and its history) into `<name>.md.enc`, and
/// every plain asset under `assets/<diary>/` into `<name>.enc`.
///
/// Each encrypted copy is read back and verified before its plain original is deleted, so an
/// interrupted run loses nothing and can simply be repeated. Failures of single files are
/// logged and counted, they do not stop the remaining files from being migrated.
pub fn encrypt_plain_diaries(folder: &Path, key: &FolderKey) -> VaultResult<MigrationReport> {
    let diary_outcomes = plain_diary_file_names(folder)?
        .into_iter()
        .map(|file_name| {
            migrate_diary(folder, key, &file_name).inspect_err(|err| {
                tracing::warn!("Could not encrypt diary {file_name}: {err}");
            })
        });
    let asset_outcomes = plain_asset_files(folder)
        .into_iter()
        .map(|(directory, file_name)| {
            migrate_asset(&directory, key, &file_name).inspect_err(|err| {
                tracing::warn!("Could not encrypt asset {directory:?}/{file_name}: {err}");
            })
        });
    let outcomes = diary_outcomes.chain(asset_outcomes).collect::<Vec<_>>();

    let migrated = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
    Ok(MigrationReport {
        migrated,
        failed: outcomes.len() - migrated,
    })
}

/// Lists the visible plain `.md` files that sit directly inside `folder`.
fn plain_diary_file_names(folder: &Path) -> VaultResult<Vec<String>> {
    Ok(visible_file_names(folder)?
        .into_iter()
        .filter(|file_name| file_name.ends_with(PLAIN_DIARY_EXTENSION))
        .collect())
}

/// Lists the names of the visible files that sit directly inside `directory`.
fn visible_file_names(directory: &Path) -> VaultResult<Vec<String>> {
    Ok(fs::read_dir(directory)
        .map_err(VaultError::io_at(directory))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|file_name| !file_name.starts_with('.'))
        .collect())
}

/// Lists the visible asset files under `assets/<diary>/` of `folder` that are not encrypted
/// yet, each with the directory it is in. A folder without assets has none.
fn plain_asset_files(folder: &Path) -> Vec<(PathBuf, String)> {
    fs::read_dir(folder.join(ASSETS_DIRECTORY))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
        .map(|entry| entry.path())
        .flat_map(|directory| {
            visible_file_names(&directory)
                .inspect_err(|err| tracing::warn!("Could not list assets: {err}"))
                .unwrap_or_default()
                .into_iter()
                .filter(|file_name| !file_name.ends_with(ENCRYPTED_SUFFIX))
                .map(move |file_name| (directory.clone(), file_name))
        })
        .collect()
}

/// Encrypts one diary with its history, then deletes the plain files.
fn migrate_diary(folder: &Path, key: &FolderKey, plain_name: &str) -> VaultResult<()> {
    let (plain_path, encrypted_name) = write_encrypted_copy(folder, key, plain_name)?;

    migrate_history(folder, key, plain_name, &encrypted_name)?;
    fs::remove_file(&plain_path).map_err(VaultError::io_at(&plain_path))
}

/// Encrypts one asset, then deletes the plain file.
fn migrate_asset(directory: &Path, key: &FolderKey, plain_name: &str) -> VaultResult<()> {
    let (plain_path, _) = write_encrypted_copy(directory, key, plain_name)?;

    fs::remove_file(&plain_path).map_err(VaultError::io_at(&plain_path))
}

/// Writes the verified encrypted copy `<plain_name>.enc` of the file `plain_name` in
/// `directory`, unless an identical copy is already there. Returns the path of the plain file
/// and the name of the encrypted one.
fn write_encrypted_copy(
    directory: &Path,
    key: &FolderKey,
    plain_name: &str,
) -> VaultResult<(PathBuf, String)> {
    let encrypted_name = format!("{plain_name}{ENCRYPTED_SUFFIX}");
    let plain_path = directory.join(plain_name);
    let encrypted_path = directory.join(&encrypted_name);
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

    Ok((plain_path, encrypted_name))
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
    fn encrypts_plain_assets_and_removes_originals() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = FolderKey::generate()?;
        let assets = dir.path().join(ASSETS_DIRECTORY).join("240101");
        fs::create_dir_all(&assets)?;
        fs::write(dir.path().join("240101.md"), "![](assets/240101/a.png)")?;
        fs::write(assets.join("a.png"), b"picture")?;

        let report = encrypt_plain_diaries(dir.path(), &key)?;

        assert_eq!(
            report,
            MigrationReport {
                migrated: 2,
                failed: 0
            }
        );
        assert!(!assets.join("a.png").exists());
        assert_eq!(read_decrypted(&assets, &key, "a.png.enc")?, b"picture");
        Ok(())
    }

    #[test]
    fn already_encrypted_and_hidden_assets_are_left_alone() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = FolderKey::generate()?;
        let assets = dir.path().join(ASSETS_DIRECTORY).join("240101");
        fs::create_dir_all(&assets)?;
        let encrypted = encrypt_file(&key, "a.png.enc", b"picture")?;
        fs::write(assets.join("a.png.enc"), &encrypted)?;
        fs::write(assets.join(".a.png.tmp"), b"staged")?;

        let report = encrypt_plain_diaries(dir.path(), &key)?;

        assert_eq!(
            report,
            MigrationReport {
                migrated: 0,
                failed: 0
            }
        );
        assert_eq!(fs::read(assets.join("a.png.enc"))?, encrypted);
        assert!(assets.join(".a.png.tmp").exists());
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
