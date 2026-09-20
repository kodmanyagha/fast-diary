use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use super::{
    atomic_write::write_atomically,
    codec::Codec,
    error::{StorageError, StorageResult},
    history::{history_directory, snapshot_if_due, HistoryPolicy},
};

#[derive(Debug, Clone)]
pub struct DiaryStore {
    base_path: PathBuf,
    history_policy: HistoryPolicy,
    codec: Codec,
}

impl DiaryStore {
    pub fn new(base_path: impl Into<PathBuf>, history_policy: HistoryPolicy, codec: Codec) -> Self {
        Self {
            base_path: base_path.into(),
            history_policy,
            codec,
        }
    }

    /// Returns the extension that new diaries of this store get.
    pub fn file_extension(&self) -> &'static str {
        self.codec.file_extension()
    }

    /// Lists the names of the diary files that this store can read.
    pub fn list_file_names(&self) -> StorageResult<Vec<String>> {
        Ok(fs::read_dir(&self.base_path)
            .map_err(StorageError::io_at(&self.base_path))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_file()))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|file_name| self.codec.accepts_file_name(file_name))
            .collect())
    }

    /// Reads the diary `file_name` as text.
    pub fn read_text(&self, file_name: &str) -> StorageResult<String> {
        let path = self.file_path(file_name)?;

        fs::read(&path)
            .map_err(StorageError::io_at(&path))
            .and_then(|content| self.codec.decode(&path, &content))
    }

    /// Creates an empty diary `file_name`, failing when it already exists.
    pub fn create(&self, file_name: &str) -> StorageResult<()> {
        let path = self.file_path(file_name)?;
        let empty_diary = self.codec.encode(&path, "")?;

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .and_then(|mut file| file.write_all(&empty_diary).and_then(|()| file.sync_all()))
            .map_err(StorageError::io_at(&path))
    }

    /// Saves `text` as the diary `file_name`, keeping the replaced content in the history.
    pub fn write_text(&self, file_name: &str, text: &str) -> StorageResult<()> {
        self.write_text_at(file_name, text, Utc::now())
    }

    /// Same as [`Self::write_text`], with the snapshot time supplied by the caller.
    fn write_text_at(&self, file_name: &str, text: &str, now: DateTime<Utc>) -> StorageResult<()> {
        let path = self.file_path(file_name)?;
        let previous_content = read_if_exists(&path)?;
        let previous_text = previous_content
            .as_deref()
            .and_then(|content| self.codec.decode(&path, content).ok());

        if previous_text.as_deref() == Some(text) {
            return Ok(());
        }

        let has_content_worth_keeping = previous_text.is_none_or(|previous| !previous.is_empty());
        if let Some(previous_content) = previous_content.filter(|_| has_content_worth_keeping) {
            let history_dir = history_directory(&self.base_path, file_name);
            if let Err(err) =
                snapshot_if_due(&self.history_policy, &history_dir, &previous_content, now)
            {
                tracing::warn!("Could not record history of {file_name}: {err}");
            }
        }

        write_atomically(&path, &self.codec.encode(&path, text)?)
    }

    /// Returns the full path of `file_name` inside the store, rejecting names that
    /// would point outside of it.
    fn file_path(&self, file_name: &str) -> StorageResult<PathBuf> {
        (Path::new(file_name).file_name() == Some(OsStr::new(file_name)))
            .then(|| self.base_path.join(file_name))
            .ok_or_else(|| StorageError::InvalidFileName(file_name.to_string()))
    }
}

/// Reads `path`, mapping a missing file to `None`.
fn read_if_exists(path: &Path) -> StorageResult<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(StorageError::io_at(path)(err)),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, io, sync::Arc};

    use chrono::TimeDelta;

    use super::*;
    use crate::vault::folder_key::FolderKey;

    fn plain_store(dir: &Path, policy: HistoryPolicy) -> DiaryStore {
        DiaryStore::new(dir, policy, Codec::Plain)
    }

    fn encrypted_store(dir: &Path, policy: HistoryPolicy) -> anyhow::Result<DiaryStore> {
        Ok(DiaryStore::new(
            dir,
            policy,
            Codec::Encrypted(Arc::new(FolderKey::generate()?)),
        ))
    }

    fn snapshot_paths(dir: &Path, file_name: &str) -> anyhow::Result<Vec<PathBuf>> {
        match fs::read_dir(history_directory(dir, file_name)) {
            Ok(entries) => Ok(entries
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<_, _>>()?),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(err.into()),
        }
    }

    fn snapshot_texts(dir: &Path, file_name: &str) -> anyhow::Result<BTreeSet<String>> {
        snapshot_paths(dir, file_name)?
            .iter()
            .map(|path| fs::read_to_string(path).map_err(anyhow::Error::from))
            .collect()
    }

    #[test]
    fn writes_and_reads_text_back() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let store = plain_store(dir.path(), HistoryPolicy::default());

        store.write_text("240101.md", "günaydın")?;

        assert_eq!(store.read_text("240101.md")?, "günaydın");
        Ok(())
    }

    #[test]
    fn create_makes_empty_file_and_refuses_duplicates() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let store = plain_store(dir.path(), HistoryPolicy::default());

        store.create("240101.md")?;

        assert_eq!(store.read_text("240101.md")?, "");
        assert!(store.create("240101.md").is_err());
        Ok(())
    }

    #[test]
    fn rejects_names_that_escape_the_store() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let store = plain_store(dir.path(), HistoryPolicy::default());

        ["../outside.md", "nested/inner.md", "..", ".", ""]
            .iter()
            .for_each(|name| {
                assert!(
                    matches!(
                        store.write_text(name, "x"),
                        Err(StorageError::InvalidFileName(_))
                    ),
                    "{name:?} should be rejected"
                );
            });
        Ok(())
    }

    #[test]
    fn rejects_invalid_utf8_content() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("240101.md"), [0xff, 0xfe, 0xfd])?;
        let store = plain_store(dir.path(), HistoryPolicy::default());

        assert!(matches!(
            store.read_text("240101.md"),
            Err(StorageError::InvalidUtf8 { .. })
        ));
        Ok(())
    }

    #[test]
    fn first_write_of_empty_file_records_no_history() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let store = plain_store(dir.path(), HistoryPolicy::default());
        store.create("240101.md")?;

        store.write_text("240101.md", "first words")?;

        assert!(snapshot_paths(dir.path(), "240101.md")?.is_empty());
        Ok(())
    }

    #[test]
    fn overwrite_keeps_previous_content_in_history() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let store = plain_store(dir.path(), HistoryPolicy::default());

        store.write_text("240101.md", "version one")?;
        store.write_text("240101.md", "version two")?;

        assert_eq!(
            snapshot_texts(dir.path(), "240101.md")?,
            BTreeSet::from(["version one".to_string()])
        );
        assert_eq!(store.read_text("240101.md")?, "version two");
        Ok(())
    }

    #[test]
    fn saving_identical_text_changes_nothing() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let store = plain_store(dir.path(), HistoryPolicy::new(5, TimeDelta::zero()));

        store.write_text("240101.md", "same")?;
        store.write_text("240101.md", "same")?;
        store.write_text("240101.md", "same")?;

        assert!(snapshot_paths(dir.path(), "240101.md")?.is_empty());
        Ok(())
    }

    #[test]
    fn keeps_only_the_newest_snapshots_of_a_long_session() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let store = plain_store(dir.path(), HistoryPolicy::new(5, TimeDelta::zero()));
        let start = Utc::now();

        (0..10).try_for_each(|index| {
            store.write_text_at(
                "240101.md",
                &format!("version {index}"),
                start + TimeDelta::seconds(index),
            )
        })?;

        let expected = (4..9)
            .map(|index| format!("version {index}"))
            .collect::<BTreeSet<_>>();
        assert_eq!(snapshot_texts(dir.path(), "240101.md")?, expected);
        assert_eq!(store.read_text("240101.md")?, "version 9");
        Ok(())
    }

    #[test]
    fn encrypted_store_never_writes_plain_text_to_disk() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let store = encrypted_store(dir.path(), HistoryPolicy::new(5, TimeDelta::zero()))?;

        store.create("240101.md.enc")?;
        store.write_text("240101.md.enc", "dear diary, top secret one")?;
        store.write_text("240101.md.enc", "dear diary, top secret two")?;

        let on_disk_files = fs::read_dir(dir.path())?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .chain(snapshot_paths(dir.path(), "240101.md.enc")?)
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        assert_eq!(on_disk_files.len(), 2);
        on_disk_files.iter().try_for_each(|path| {
            let content = fs::read(path)?;
            anyhow::ensure!(!content.windows(6).any(|window| window == b"secret"));
            anyhow::ensure!(content.starts_with(b"FDRY"));
            Ok(())
        })?;
        assert_eq!(
            store.read_text("240101.md.enc")?,
            "dear diary, top secret two"
        );
        Ok(())
    }

    #[test]
    fn encrypted_history_snapshots_decrypt_to_previous_versions() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let key = Arc::new(FolderKey::generate()?);
        let codec = Codec::Encrypted(key);
        let store = DiaryStore::new(dir.path(), HistoryPolicy::default(), codec.clone());

        store.write_text("240101.md.enc", "older")?;
        store.write_text("240101.md.enc", "newer")?;

        let snapshots = snapshot_paths(dir.path(), "240101.md.enc")?;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(
            codec.decode(&dir.path().join("240101.md.enc"), &fs::read(&snapshots[0])?)?,
            "older"
        );
        Ok(())
    }

    #[test]
    fn encrypted_store_rejects_files_of_another_key() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        encrypted_store(dir.path(), HistoryPolicy::default())?.write_text("a.md.enc", "text")?;

        let other = encrypted_store(dir.path(), HistoryPolicy::default())?;

        assert!(matches!(
            other.read_text("a.md.enc"),
            Err(StorageError::Cipher { .. })
        ));
        Ok(())
    }

    #[test]
    fn lists_only_files_the_codec_accepts() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        ["240101.md", "240102.md.enc", ".encrypted", ".240103.md.tmp"]
            .iter()
            .try_for_each(|name| fs::write(dir.path().join(name), "x"))?;
        fs::create_dir(dir.path().join(".history"))?;
        fs::create_dir(dir.path().join("folder.md"))?;

        let plain = plain_store(dir.path(), HistoryPolicy::default());
        let encrypted = encrypted_store(dir.path(), HistoryPolicy::default())?;

        assert_eq!(plain.list_file_names()?, vec!["240101.md".to_string()]);
        assert_eq!(
            encrypted.list_file_names()?,
            vec!["240102.md.enc".to_string()]
        );
        Ok(())
    }
}
