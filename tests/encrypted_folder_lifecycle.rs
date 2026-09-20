use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use chrono::TimeDelta;
use fast_diary::{
    storage::{codec::Codec, diary_store::DiaryStore, history::HistoryPolicy},
    vault::{
        error::VaultError,
        folder::{change_password, create_encrypted_folder, is_encrypted_folder, unlock_folder},
        kdf::KdfParams,
        migration::encrypt_plain_diaries,
    },
};

const FAST_KDF_PARAMS: KdfParams = KdfParams {
    memory_kib: 64,
    iterations: 1,
    parallelism: 1,
};
const PASSWORD: &str = "correct horse";
const DIARY: &str = "240101120000.md.enc";
const WRITE_SPACING: Duration = Duration::from_millis(3);

fn all_files(directory: &Path) -> anyhow::Result<Vec<PathBuf>> {
    fs::read_dir(directory)?
        .map(|entry| Ok(entry?.path()))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .map(|path| {
            if path.is_dir() {
                all_files(&path)
            } else {
                Ok(vec![path])
            }
        })
        .collect::<anyhow::Result<Vec<_>>>()
        .map(|nested| nested.into_iter().flatten().collect())
}

fn open_store(folder: &Path, password: &str) -> anyhow::Result<DiaryStore> {
    let key = unlock_folder(folder, password)?;
    Ok(DiaryStore::new(
        folder,
        HistoryPolicy::new(5, TimeDelta::zero()),
        Codec::Encrypted(Arc::new(key)),
    ))
}

#[test]
fn diaries_and_history_of_an_encrypted_folder_are_unreadable_without_the_password(
) -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    fs::write(folder.path().join("231231120000.md"), "secret old entry")?;

    let key = create_encrypted_folder(folder.path(), PASSWORD, PASSWORD, &FAST_KDF_PARAMS)?;
    let report = encrypt_plain_diaries(folder.path(), &key)?;
    assert_eq!(report.migrated, 1);

    let store = open_store(folder.path(), PASSWORD)?;
    store.create(DIARY)?;
    (0..8).try_for_each(|version| {
        thread::sleep(WRITE_SPACING);
        store.write_text(DIARY, &format!("secret entry, version {version}"))
    })?;

    let files = all_files(folder.path())?;
    let diaries_and_snapshots = files
        .iter()
        .filter(|path| !path.ends_with(".encrypted"))
        .collect::<Vec<_>>();
    assert!(!diaries_and_snapshots.is_empty());
    diaries_and_snapshots.iter().try_for_each(|path| {
        let content = fs::read(path)?;
        anyhow::ensure!(content.starts_with(b"FDRY"), "{path:?} is not encrypted");
        anyhow::ensure!(
            !content.windows(6).any(|window| window == b"secret"),
            "{path:?} leaks plain text"
        );
        Ok(())
    })?;
    assert!(!fs::read_to_string(folder.path().join(".encrypted"))?.contains("secret"));

    let history_files = all_files(&folder.path().join(".history").join(DIARY))?;
    assert_eq!(history_files.len(), HistoryPolicy::MIN_KEEP_COUNT);
    assert_eq!(store.read_text(DIARY)?, "secret entry, version 7");
    assert_eq!(store.read_text("231231120000.md.enc")?, "secret old entry");
    Ok(())
}

#[test]
fn changing_the_password_leaves_encrypted_diaries_untouched() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;
    create_encrypted_folder(folder.path(), PASSWORD, PASSWORD, &FAST_KDF_PARAMS)?;
    let store = open_store(folder.path(), PASSWORD)?;
    store.write_text(DIARY, "kept as is")?;
    let diary_bytes_before = fs::read(folder.path().join(DIARY))?;
    let key_file_before = fs::read(folder.path().join(".encrypted"))?;

    change_password(
        folder.path(),
        PASSWORD,
        "battery staple",
        "battery staple",
        &FAST_KDF_PARAMS,
    )?;

    assert_eq!(fs::read(folder.path().join(DIARY))?, diary_bytes_before);
    assert_ne!(fs::read(folder.path().join(".encrypted"))?, key_file_before);
    assert!(matches!(
        unlock_folder(folder.path(), PASSWORD),
        Err(VaultError::WrongPassword)
    ));
    assert_eq!(
        open_store(folder.path(), "battery staple")?.read_text(DIARY)?,
        "kept as is"
    );
    Ok(())
}

#[test]
fn folder_without_a_key_file_is_not_encrypted() -> anyhow::Result<()> {
    let folder = tempfile::tempdir()?;

    assert!(!is_encrypted_folder(folder.path()));
    create_encrypted_folder(folder.path(), PASSWORD, PASSWORD, &FAST_KDF_PARAMS)?;
    assert!(is_encrypted_folder(folder.path()));
    Ok(())
}
