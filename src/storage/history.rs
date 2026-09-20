use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use chrono::{DateTime, TimeDelta, Utc};

use super::{
    atomic_write::write_atomically,
    error::{StorageError, StorageResult},
};

const HISTORY_DIRECTORY_NAME: &str = ".history";
const SNAPSHOT_EXTENSION: &str = "snap";
const DEFAULT_KEEP_COUNT: usize = 10;
const DEFAULT_MIN_SNAPSHOT_INTERVAL_MINUTES: i64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryPolicy {
    keep_count: usize,
    min_snapshot_interval: TimeDelta,
}

impl HistoryPolicy {
    pub const MIN_KEEP_COUNT: usize = 5;

    /// Creates a policy that keeps at least [`Self::MIN_KEEP_COUNT`] snapshots, no matter
    /// what `keep_count` is requested.
    pub fn new(keep_count: usize, min_snapshot_interval: TimeDelta) -> Self {
        Self {
            keep_count: keep_count.max(Self::MIN_KEEP_COUNT),
            min_snapshot_interval,
        }
    }
}

impl Default for HistoryPolicy {
    fn default() -> Self {
        Self::new(
            DEFAULT_KEEP_COUNT,
            TimeDelta::minutes(DEFAULT_MIN_SNAPSHOT_INTERVAL_MINUTES),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    path: PathBuf,
    taken_at_millis: i64,
}

impl Snapshot {
    /// Parses a snapshot from its path (`<unix millis>.snap`), ignoring any other file.
    fn from_path(path: PathBuf) -> Option<Self> {
        path.extension().filter(|ext| *ext == SNAPSHOT_EXTENSION)?;
        let taken_at_millis = path.file_stem()?.to_str()?.parse::<i64>().ok()?;

        Some(Self {
            path,
            taken_at_millis,
        })
    }
}

/// Returns the directory that holds the snapshots of `file_name` inside `base_path`.
pub fn history_directory(base_path: &Path, file_name: &str) -> PathBuf {
    base_path.join(HISTORY_DIRECTORY_NAME).join(file_name)
}

/// Stores `previous_content` as a new snapshot when the latest snapshot is older than the
/// policy interval, then removes the oldest snapshots beyond the policy's keep count.
pub fn snapshot_if_due(
    policy: &HistoryPolicy,
    history_dir: &Path,
    previous_content: &[u8],
    now: DateTime<Utc>,
) -> StorageResult<()> {
    let is_due = list_snapshots(history_dir)?.last().is_none_or(|latest| {
        !(0..policy.min_snapshot_interval.num_milliseconds())
            .contains(&(now.timestamp_millis() - latest.taken_at_millis))
    });

    if is_due {
        write_snapshot(history_dir, previous_content, now)?;
    }

    prune_snapshots(policy, history_dir)
}

/// Lists the paths of the snapshot files in `history_dir` from the oldest to the newest.
pub fn snapshot_file_paths(history_dir: &Path) -> StorageResult<Vec<PathBuf>> {
    list_snapshots(history_dir).map(|snapshots| {
        snapshots
            .into_iter()
            .map(|snapshot| snapshot.path)
            .collect()
    })
}

/// Writes `content` into `history_dir` as a snapshot taken at `now`.
fn write_snapshot(history_dir: &Path, content: &[u8], now: DateTime<Utc>) -> StorageResult<()> {
    fs::create_dir_all(history_dir).map_err(StorageError::io_at(history_dir))?;

    let snapshot_path = history_dir.join(format!(
        "{:013}.{SNAPSHOT_EXTENSION}",
        now.timestamp_millis()
    ));
    write_atomically(&snapshot_path, content)
}

/// Lists the snapshots in `history_dir` from the oldest to the newest.
fn list_snapshots(history_dir: &Path) -> StorageResult<Vec<Snapshot>> {
    match fs::read_dir(history_dir) {
        Ok(entries) => Ok(entries
            .filter_map(Result::ok)
            .filter_map(|entry| Snapshot::from_path(entry.path()))
            .map(|snapshot| (snapshot.taken_at_millis, snapshot))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(StorageError::io_at(history_dir)(err)),
    }
}

/// Deletes the oldest snapshots so that at most `policy.keep_count` remain.
fn prune_snapshots(policy: &HistoryPolicy, history_dir: &Path) -> StorageResult<()> {
    let snapshots = list_snapshots(history_dir)?;
    let excess_count = snapshots.len().saturating_sub(policy.keep_count);

    snapshots
        .iter()
        .take(excess_count)
        .try_for_each(|snapshot| {
            fs::remove_file(&snapshot.path).map_err(StorageError::io_at(&snapshot.path))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_contents(history_dir: &Path) -> anyhow::Result<Vec<Vec<u8>>> {
        list_snapshots(history_dir)?
            .iter()
            .map(|snapshot| fs::read(&snapshot.path).map_err(Into::into))
            .collect()
    }

    #[test]
    fn creates_first_snapshot_when_none_exist() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let policy = HistoryPolicy::default();

        snapshot_if_due(&policy, dir.path(), b"v1", Utc::now())?;

        assert_eq!(snapshot_contents(dir.path())?, vec![b"v1".to_vec()]);
        Ok(())
    }

    #[test]
    fn skips_snapshot_when_latest_is_recent() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let policy = HistoryPolicy::new(10, TimeDelta::minutes(2));
        let start = Utc::now();

        snapshot_if_due(&policy, dir.path(), b"v1", start)?;
        snapshot_if_due(&policy, dir.path(), b"v2", start + TimeDelta::seconds(119))?;

        assert_eq!(snapshot_contents(dir.path())?, vec![b"v1".to_vec()]);
        Ok(())
    }

    #[test]
    fn creates_snapshot_when_interval_has_passed() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let policy = HistoryPolicy::new(10, TimeDelta::minutes(2));
        let start = Utc::now();

        snapshot_if_due(&policy, dir.path(), b"v1", start)?;
        snapshot_if_due(&policy, dir.path(), b"v2", start + TimeDelta::minutes(2))?;

        assert_eq!(
            snapshot_contents(dir.path())?,
            vec![b"v1".to_vec(), b"v2".to_vec()]
        );
        Ok(())
    }

    #[test]
    fn creates_snapshot_when_clock_moved_backwards() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let policy = HistoryPolicy::new(10, TimeDelta::minutes(2));
        let start = Utc::now();

        snapshot_if_due(&policy, dir.path(), b"v1", start)?;
        snapshot_if_due(&policy, dir.path(), b"v2", start - TimeDelta::hours(1))?;

        assert_eq!(
            snapshot_contents(dir.path())?,
            vec![b"v2".to_vec(), b"v1".to_vec()]
        );
        Ok(())
    }

    #[test]
    fn prunes_oldest_snapshots_beyond_keep_count() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let policy = HistoryPolicy::new(6, TimeDelta::zero());
        let start = Utc::now();

        (0..9).try_for_each(|index| {
            snapshot_if_due(
                &policy,
                dir.path(),
                format!("v{index}").as_bytes(),
                start + TimeDelta::seconds(index),
            )
        })?;

        let expected = (3..9)
            .map(|index| format!("v{index}").into_bytes())
            .collect::<Vec<_>>();
        assert_eq!(snapshot_contents(dir.path())?, expected);
        Ok(())
    }

    #[test]
    fn never_keeps_fewer_than_minimum_snapshots() {
        let policy = HistoryPolicy::new(1, TimeDelta::zero());

        assert_eq!(policy.keep_count, HistoryPolicy::MIN_KEEP_COUNT);
    }

    #[test]
    fn ignores_files_that_are_not_snapshots() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("notes.txt"), "not a snapshot")?;
        fs::write(dir.path().join("abc.snap"), "bad name")?;
        let policy = HistoryPolicy::new(5, TimeDelta::zero());

        snapshot_if_due(&policy, dir.path(), b"v1", Utc::now())?;

        assert_eq!(snapshot_contents(dir.path())?, vec![b"v1".to_vec()]);
        Ok(())
    }

    #[test]
    fn missing_history_directory_lists_no_snapshots() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;

        assert_eq!(list_snapshots(&dir.path().join("absent"))?, Vec::new());
        Ok(())
    }

    #[test]
    fn history_directory_is_scoped_per_file() {
        assert_eq!(
            history_directory(Path::new("/diaries"), "240101.md"),
            PathBuf::from("/diaries/.history/240101.md")
        );
    }
}
