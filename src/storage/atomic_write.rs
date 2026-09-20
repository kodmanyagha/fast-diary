use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use super::error::{StorageError, StorageResult};

/// Replaces `target` with `content` so that a crash at any point leaves either the
/// complete old file or the complete new file, never a truncated one.
pub fn write_atomically(target: &Path, content: &[u8]) -> StorageResult<()> {
    let temp_path = temp_path_for(target)?;

    write_and_sync(&temp_path, content)?;
    fs::rename(&temp_path, target).map_err(StorageError::io_at(target))?;

    if let Err(err) = sync_parent_directory(target) {
        tracing::debug!("Could not sync parent directory of {target:?}: {err}");
    }

    Ok(())
}

/// Returns the hidden sibling path (`.<file name>.tmp`) that staged content is written to.
fn temp_path_for(target: &Path) -> StorageResult<PathBuf> {
    target
        .file_name()
        .map(|file_name| target.with_file_name(format!(".{}.tmp", file_name.to_string_lossy())))
        .ok_or_else(|| StorageError::InvalidFileName(target.display().to_string()))
}

/// Writes `content` to `path` and flushes it to stable storage.
fn write_and_sync(path: &Path, content: &[u8]) -> StorageResult<()> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .and_then(|mut file| file.write_all(content).and_then(|()| file.sync_all()))
        .map_err(StorageError::io_at(path))
}

/// Flushes the directory entry of `target` so the rename survives a power loss.
#[cfg(unix)]
fn sync_parent_directory(target: &Path) -> io::Result<()> {
    target
        .parent()
        .map_or(Ok(()), |parent| File::open(parent)?.sync_all())
}

/// Directory syncing is not supported on this platform, so it is skipped.
#[cfg(not(unix))]
fn sync_parent_directory(_target: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn creates_missing_file() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("diary.md");

        write_atomically(&target, b"hello")?;

        assert_eq!(fs::read(&target)?, b"hello");
        Ok(())
    }

    #[test]
    fn replaces_existing_content_completely() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("diary.md");
        fs::write(&target, "a much longer previous content")?;

        write_atomically(&target, b"short")?;

        assert_eq!(fs::read(&target)?, b"short");
        Ok(())
    }

    #[test]
    fn leaves_no_temporary_file_behind() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("diary.md");

        write_atomically(&target, b"one")?;
        write_atomically(&target, b"two")?;

        let file_names = fs::read_dir(dir.path())?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(file_names, vec![OsString::from("diary.md")]);
        Ok(())
    }

    #[test]
    fn fails_when_parent_directory_is_missing() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("missing").join("diary.md");

        assert!(write_atomically(&target, b"content").is_err());
        Ok(())
    }
}
