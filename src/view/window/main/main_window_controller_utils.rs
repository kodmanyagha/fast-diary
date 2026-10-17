use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use im::Vector;
use zeroize::Zeroize;

use super::folder_command_error::FolderCommandError;
use crate::{
    config::settings::Settings,
    modal::{
        app_state::{AppState, FolderProtection, PasswordChangeForm},
        app_state_utils::diaries_compare_rev,
        state::{
            app_pages::AppPages, current_diary::CurrentDiary, diary_list_item::DiaryListItem,
            pending_confirmation::PendingConfirmation,
        },
    },
    storage::diary_store::DiaryStore,
    utils::diary::summarize,
    vault::{
        folder::{change_password, create_encrypted_folder, is_encrypted_folder, unlock_folder},
        folder_key::FolderKey,
        kdf::KdfParams,
        migration::encrypt_plain_diaries,
    },
};

/// Detects whether the selected folder is encrypted and drops everything that belonged to the
/// previously selected folder: its key, the typed passwords and the status message.
pub fn refresh_folder_protection(app_state: &mut AppState) {
    app_state.folder_protection = app_state
        .diary_base_path
        .as_deref()
        .map(Path::new)
        .filter(|folder| is_encrypted_folder(folder))
        .map_or(FolderProtection::Plain, |_| FolderProtection::Encrypted);
    app_state.folder_key = None;
    app_state.status_message.clear();
    clear_typed_passwords(app_state);
}

/// Opens the selected folder: an encrypted folder is unlocked with the typed password first.
pub fn open_selected_folder(app_state: &mut AppState) -> anyhow::Result<()> {
    if app_state.folder_protection == FolderProtection::Encrypted {
        let folder = selected_folder(app_state)?;
        let folder_key = unlock_folder(&folder, &app_state.password)?;

        log_migration(&folder, &folder_key);
        app_state.folder_key = Some(Arc::new(folder_key));
    }

    show_diary_page(app_state);
    Ok(())
}

/// Encrypts the selected plain folder with the typed password, converting the diaries it
/// already contains, then opens it.
pub fn encrypt_selected_folder(app_state: &mut AppState) -> anyhow::Result<()> {
    let folder = selected_folder(app_state)?;
    let folder_key = create_encrypted_folder(
        &folder,
        &app_state.password,
        &app_state.password_confirmation,
        &KdfParams::RECOMMENDED,
    )?;
    app_state.folder_protection = FolderProtection::Encrypted;
    clear_typed_passwords(app_state);

    let report = encrypt_plain_diaries(&folder, &folder_key)
        .map_err(FolderCommandError::DiariesNotConverted)?;
    if report.failed > 0 {
        return Err(FolderCommandError::SomeDiariesNotConverted {
            failed: report.failed,
        }
        .into());
    }

    app_state.folder_key = Some(Arc::new(folder_key));
    show_diary_page(app_state);
    Ok(())
}

/// Changes the password of the selected encrypted folder using the settings form.
pub fn change_folder_password(app_state: &mut AppState) -> anyhow::Result<()> {
    let folder = selected_folder(app_state)?;
    let form = &app_state.password_change;

    change_password(
        &folder,
        &form.current_password,
        &form.new_password,
        &form.confirmation,
        &KdfParams::RECOMMENDED,
    )?;

    clear_typed_passwords(app_state);
    Ok(())
}

/// Forgets the folder key and every opened diary, then returns to the folder selection page.
pub fn lock_folder(app_state: &mut AppState) {
    app_state.txt_diary.zeroize();
    app_state.current_diary = CurrentDiary::new();
    app_state.diaries.clear();
    refresh_folder_protection(app_state);
    app_state.page = AppPages::Main;
}

/// Removes what `confirmation` asks for from the recent folders and unselects the selected
/// folder when it is no longer listed. Returns the settings that hold the new recent folders.
pub fn forget_recent_folders(
    app_state: &mut AppState,
    confirmation: &PendingConfirmation,
) -> Settings {
    let mut settings = Settings::from(&*app_state);
    match confirmation {
        PendingConfirmation::RemoveRecentFolder(folder_path) => {
            settings.remove_recent_folder(folder_path)
        }
        PendingConfirmation::ClearRecentFolders => settings.clear_recent_folders(),
    }

    app_state.recent_folders = settings.recent_folders.iter().cloned().collect();
    app_state.diary_base_path = app_state.selected_recent_folder().cloned();
    settings
}

/// Lists the diaries of `store` with their summaries, newest first. Files that are not
/// diaries or cannot be read are skipped.
pub fn load_diary_items(store: &DiaryStore) -> anyhow::Result<Vector<DiaryListItem>> {
    let mut diary_items = store
        .list_file_names()?
        .iter()
        .filter_map(|file_name| load_diary_item(store, file_name))
        .collect::<Vector<_>>();
    diary_items.sort_by(diaries_compare_rev);

    Ok(diary_items)
}

/// Reads one diary and builds its list item, or `None` when it is not a readable diary.
fn load_diary_item(store: &DiaryStore, file_name: &str) -> Option<DiaryListItem> {
    let text = store
        .read_text(file_name)
        .inspect_err(|err| tracing::warn!("Could not read diary {file_name}: {err}"))
        .ok()?;

    DiaryListItem::from_file_name(file_name, summarize(&text))
        .inspect_err(|err| tracing::debug!("Skipping {file_name}: {err}"))
        .ok()
}

/// Returns the selected folder or an error telling the user to select one.
fn selected_folder(app_state: &AppState) -> Result<PathBuf, FolderCommandError> {
    app_state
        .diary_base_path
        .as_ref()
        .map(PathBuf::from)
        .ok_or(FolderCommandError::NoFolderSelected)
}

/// Clears the passwords the user typed on any page.
fn clear_typed_passwords(app_state: &mut AppState) {
    app_state.password.zeroize();
    app_state.password_confirmation.zeroize();
    app_state.password_change.current_password.zeroize();
    app_state.password_change.new_password.zeroize();
    app_state.password_change.confirmation.zeroize();
    app_state.password_change = PasswordChangeForm::default();
}

/// Switches to the diary page and clears the status message and typed passwords.
fn show_diary_page(app_state: &mut AppState) {
    app_state.status_message.clear();
    clear_typed_passwords(app_state);
    app_state.page = AppPages::Diary;
}

/// Encrypts plain diaries that were put into an already encrypted folder, logging the outcome.
fn log_migration(folder: &Path, folder_key: &FolderKey) {
    match encrypt_plain_diaries(folder, folder_key) {
        Ok(report) if report.migrated > 0 || report.failed > 0 => tracing::info!(
            "Encrypted {} plain diaries, {} failed.",
            report.migrated,
            report.failed
        ),
        Ok(_) => {}
        Err(err) => tracing::warn!("Could not look for plain diaries to encrypt: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::anyhow;

    use super::*;

    fn state_for(folder: &Path) -> AppState {
        let mut app_state = AppState::new();
        app_state.diary_base_path = Some(folder.display().to_string());
        refresh_folder_protection(&mut app_state);
        app_state
    }

    fn type_password(app_state: &mut AppState, password: &str) {
        app_state.password = password.to_string();
        app_state.password_confirmation = password.to_string();
    }

    #[test]
    fn plain_folder_opens_without_password() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut app_state = state_for(dir.path());

        open_selected_folder(&mut app_state)?;

        assert_eq!(app_state.folder_protection, FolderProtection::Plain);
        assert_eq!(app_state.page, AppPages::Diary);
        assert!(app_state.diary_store().is_some());
        Ok(())
    }

    #[test]
    fn encrypting_a_folder_converts_existing_diaries_and_opens_it() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("240101120000.md"), "old diary")?;
        let mut app_state = state_for(dir.path());
        type_password(&mut app_state, "123456");

        encrypt_selected_folder(&mut app_state)?;

        assert_eq!(app_state.folder_protection, FolderProtection::Encrypted);
        assert_eq!(app_state.page, AppPages::Diary);
        assert!(app_state.password.is_empty());
        assert!(!dir.path().join("240101120000.md").exists());
        let store = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("store missing"))?;
        assert_eq!(
            store.read_text("240101120000.md.enc")?,
            "old diary".to_string()
        );
        Ok(())
    }

    #[test]
    fn locked_encrypted_folder_gives_no_store_and_needs_the_right_password() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut app_state = state_for(dir.path());
        type_password(&mut app_state, "123456");
        encrypt_selected_folder(&mut app_state)?;
        let store = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("store missing"))?;
        store.create("240101120000.md.enc")?;
        store.write_text("240101120000.md.enc", "my secret")?;

        lock_folder(&mut app_state);

        assert_eq!(app_state.page, AppPages::Main);
        assert_eq!(app_state.folder_protection, FolderProtection::Encrypted);
        assert!(app_state.folder_key.is_none());
        assert!(app_state.diary_store().is_none());
        assert!(app_state.txt_diary.is_empty());

        app_state.password = "wrong!".to_string();
        assert!(open_selected_folder(&mut app_state).is_err());
        assert!(app_state.diary_store().is_none());
        assert_eq!(app_state.page, AppPages::Main);

        app_state.password = "123456".to_string();
        open_selected_folder(&mut app_state)?;
        let reopened = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("store missing"))?;
        assert_eq!(reopened.read_text("240101120000.md.enc")?, "my secret");
        assert_eq!(app_state.page, AppPages::Diary);
        Ok(())
    }

    #[test]
    fn plain_diary_dropped_into_an_encrypted_folder_is_encrypted_on_unlock() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut app_state = state_for(dir.path());
        type_password(&mut app_state, "123456");
        encrypt_selected_folder(&mut app_state)?;
        lock_folder(&mut app_state);
        fs::write(dir.path().join("240202120000.md"), "dropped in")?;

        app_state.password = "123456".to_string();
        open_selected_folder(&mut app_state)?;

        assert!(!dir.path().join("240202120000.md").exists());
        let store = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("store missing"))?;
        assert_eq!(store.read_text("240202120000.md.enc")?, "dropped in");
        Ok(())
    }

    #[test]
    fn changed_password_opens_the_folder_and_old_one_does_not() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut app_state = state_for(dir.path());
        type_password(&mut app_state, "123456");
        encrypt_selected_folder(&mut app_state)?;
        let store = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("store missing"))?;
        store.write_text("240101120000.md.enc", "kept")?;
        let diary_before = fs::read(dir.path().join("240101120000.md.enc"))?;

        app_state.password_change = PasswordChangeForm {
            current_password: "123456".to_string(),
            new_password: "a better secret".to_string(),
            confirmation: "a better secret".to_string(),
        };
        change_folder_password(&mut app_state)?;
        lock_folder(&mut app_state);

        assert_eq!(
            diary_before,
            fs::read(dir.path().join("240101120000.md.enc"))?
        );
        app_state.password = "123456".to_string();
        assert!(open_selected_folder(&mut app_state).is_err());
        app_state.password = "a better secret".to_string();
        open_selected_folder(&mut app_state)?;
        let reopened = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("store missing"))?;
        assert_eq!(reopened.read_text("240101120000.md.enc")?, "kept");
        Ok(())
    }

    #[test]
    fn selecting_another_folder_forgets_the_previous_key() -> anyhow::Result<()> {
        let encrypted_dir = tempfile::tempdir()?;
        let plain_dir = tempfile::tempdir()?;
        let mut app_state = state_for(encrypted_dir.path());
        type_password(&mut app_state, "123456");
        encrypt_selected_folder(&mut app_state)?;

        app_state.diary_base_path = Some(plain_dir.path().display().to_string());
        refresh_folder_protection(&mut app_state);

        assert_eq!(app_state.folder_protection, FolderProtection::Plain);
        assert!(app_state.folder_key.is_none());
        Ok(())
    }

    fn state_with_recent_folders(selected: &str, recent_folders: &[&str]) -> AppState {
        let mut app_state = AppState::new();
        app_state.diary_base_path = Some(selected.to_string());
        app_state.recent_folders = recent_folders.iter().map(|path| path.to_string()).collect();
        app_state
    }

    fn recent_folders_of(app_state: &AppState) -> Vec<&str> {
        app_state
            .recent_folders
            .iter()
            .map(String::as_str)
            .collect()
    }

    #[test]
    fn removing_the_selected_recent_folder_unselects_it() {
        let mut app_state = state_with_recent_folders("/b", &["/a", "/b", "/c"]);

        let settings = forget_recent_folders(
            &mut app_state,
            &PendingConfirmation::RemoveRecentFolder("/b".to_string()),
        );

        assert_eq!(recent_folders_of(&app_state), vec!["/a", "/c"]);
        assert_eq!(settings.recent_folders, vec!["/a", "/c"]);
        assert_eq!(app_state.diary_base_path, None);
    }

    #[test]
    fn removing_another_recent_folder_keeps_the_selection() {
        let mut app_state = state_with_recent_folders("/b", &["/a", "/b"]);

        forget_recent_folders(
            &mut app_state,
            &PendingConfirmation::RemoveRecentFolder("/a".to_string()),
        );

        assert_eq!(recent_folders_of(&app_state), vec!["/b"]);
        assert_eq!(app_state.diary_base_path.as_deref(), Some("/b"));
    }

    #[test]
    fn clearing_the_recent_folders_unselects_the_folder() {
        let mut app_state = state_with_recent_folders("/a", &["/a", "/b"]);

        let settings =
            forget_recent_folders(&mut app_state, &PendingConfirmation::ClearRecentFolders);

        assert!(app_state.recent_folders.is_empty());
        assert!(settings.recent_folders.is_empty());
        assert_eq!(app_state.diary_base_path, None);
    }

    #[test]
    fn diary_list_is_sorted_newest_first_and_summarized() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("240101120000.md"), "first day\nmore")?;
        fs::write(dir.path().join("240301120000.md"), "third month")?;
        fs::write(dir.path().join("README.md"), "not a diary")?;
        let app_state = state_for(dir.path());
        let store = app_state
            .diary_store()
            .ok_or_else(|| anyhow!("store missing"))?;

        let items = load_diary_items(&store)?;

        assert_eq!(
            items
                .iter()
                .map(|item| (item.file_name.as_str(), item.summary.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("240301120000.md", "third month"),
                ("240101120000.md", "first day")
            ]
        );
        Ok(())
    }
}
