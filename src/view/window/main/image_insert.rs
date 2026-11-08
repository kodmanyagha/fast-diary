use std::path::Path;

use anyhow::Context;
use chrono::Local;
use druid::{commands, Command, FileDialogOptions, FileSpec};

use super::folder_command_error::FolderCommandError;
use crate::{
    modal::{
        app_state::{AppState, OpenFilePurpose},
        state::{editor_mode::EditorMode, language::Language},
    },
    storage::{asset_name::IMAGE_EXTENSIONS, asset_store::AssetStore},
    utils::{
        diary::{image_markdown, with_block_inserted},
        localization::translate,
    },
};

const IMAGE_FILE_SPEC: FileSpec = FileSpec::new("PNG, JPEG, WebP, GIF, BMP", IMAGE_EXTENSIONS);

/// Where the markdown of an inserted image has to go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertedImage {
    /// The editor is shown, so it puts the markdown at its caret.
    ForEditor(String),
    /// The editor is hidden, so the markdown was added to the end of the diary.
    AppendedToDiary,
}

/// Returns the options of the dialog that picks an image to insert into the opened diary.
pub fn image_dialog_options(language: Language) -> FileDialogOptions {
    FileDialogOptions::new()
        .allowed_types(vec![IMAGE_FILE_SPEC])
        .default_type(IMAGE_FILE_SPEC)
        .title(translate(language, "dialog-image-title"))
        .button_text(translate(language, "dialog-image-select"))
}

/// Notes that the next opened file is an image to insert, and returns the command that asks
/// the user for it.
pub fn image_insert_request(app_state: &mut AppState) -> Command {
    app_state.open_file_purpose = OpenFilePurpose::ImageInsert;
    commands::SHOW_OPEN_PANEL.with(image_dialog_options(app_state.language))
}

/// The name that images pasted from the clipboard are saved under, before the moment of
/// pasting is added to it.
pub const PASTED_IMAGE_NAME: &str = "screenshot.png";

const IMAGE_SEPARATOR: &str = "\n\n";

/// Returns the asset store of the opened folder together with the file name of the opened
/// diary, which the assets are kept for.
fn asset_target(app_state: &AppState) -> anyhow::Result<(AssetStore, String)> {
    if !app_state.current_diary.is_selected {
        return Err(FolderCommandError::NoDiaryOpened.into());
    }
    let store = app_state
        .asset_store()
        .ok_or(FolderCommandError::NoFolderSelected)?;
    Ok((store, app_state.current_diary.diary.file_name.clone()))
}

/// Puts `markdown` into the diary: at the end right away while the editor is hidden, otherwise
/// it is handed back for the editor to put at its caret.
fn place_markdown(app_state: &mut AppState, markdown: String) -> InsertedImage {
    match app_state.editor_mode {
        EditorMode::Preview => {
            let end = app_state.txt_diary.len();
            app_state.txt_diary = with_block_inserted(&app_state.txt_diary, end..end, &markdown).0;
            InsertedImage::AppendedToDiary
        }
        EditorMode::Edit | EditorMode::Split => InsertedImage::ForEditor(markdown),
    }
}

/// Copies the image at `source` into the assets of the opened diary and returns the markdown
/// that shows it.
fn import_image_markdown(
    store: &AssetStore,
    diary_file_name: &str,
    source: &Path,
) -> anyhow::Result<String> {
    let reference = store
        .import_image(diary_file_name, source, Local::now().naive_local())
        .with_context(|| format!("Could not copy the image {source:?}."))?;
    let description = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(image_markdown(&description, &reference))
}

/// Copies the images at `sources` into the assets of the opened diary and puts the markdown
/// that shows them into the diary, each image in a paragraph of its own.
pub fn insert_images(
    app_state: &mut AppState,
    sources: &[impl AsRef<Path>],
) -> anyhow::Result<InsertedImage> {
    let (store, diary_file_name) = asset_target(app_state)?;
    let markdown = sources
        .iter()
        .map(|source| import_image_markdown(&store, &diary_file_name, source.as_ref()))
        .collect::<anyhow::Result<Vec<_>>>()?
        .join(IMAGE_SEPARATOR);

    Ok(place_markdown(app_state, markdown))
}

/// Copies the image at `source` into the assets of the opened diary and puts the markdown that
/// shows it into the diary.
pub fn insert_image(app_state: &mut AppState, source: &Path) -> anyhow::Result<InsertedImage> {
    insert_images(app_state, &[source])
}

/// Saves the PNG image `png`, such as a screenshot pasted from the clipboard, into the assets
/// of the opened diary as [`PASTED_IMAGE_NAME`] and puts the markdown that shows it into the
/// diary.
pub fn insert_image_data(app_state: &mut AppState, png: &[u8]) -> anyhow::Result<InsertedImage> {
    let (store, diary_file_name) = asset_target(app_state)?;
    let original_name = Path::new(PASTED_IMAGE_NAME);
    let reference = store
        .import_image_data(
            &diary_file_name,
            original_name,
            png,
            Local::now().naive_local(),
        )
        .context("Could not save the pasted image.")?;
    let description = original_name
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();

    Ok(place_markdown(
        app_state,
        image_markdown(&description, &reference),
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::modal::state::diary_list_item::DiaryListItem;

    fn state_with_diary(folder: &Path, mode: EditorMode) -> AppState {
        let mut app_state = AppState::new();
        app_state.diary_base_path = Some(folder.to_string_lossy().into_owned());
        app_state.current_diary = DiaryListItem::new()
            .with_file_name("251108171400.md".to_string())
            .into();
        app_state.editor_mode = mode;
        app_state.txt_diary = "Dear diary".to_string();
        app_state
    }

    const ONE_PIXEL_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fn picture_named(dir: &Path, name: &str) -> anyhow::Result<std::path::PathBuf> {
        let path = dir.join(name);
        fs::write(&path, ONE_PIXEL_PNG)?;
        Ok(path)
    }

    fn picture(dir: &Path) -> anyhow::Result<std::path::PathBuf> {
        picture_named(dir, "Güzel Gün.png")
    }

    #[test]
    fn the_editor_gets_the_markdown_of_the_copied_image() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let mut app_state = state_with_diary(folder.path(), EditorMode::Split);

        let inserted = insert_image(&mut app_state, &picture(outside.path())?)?;

        let InsertedImage::ForEditor(markdown) = inserted else {
            anyhow::bail!("the editor should insert the image");
        };
        assert!(markdown.starts_with("![Güzel Gün](assets/251108171400/guzel_gun_"));
        assert!(markdown.ends_with(".png)"));
        assert_eq!(app_state.txt_diary, "Dear diary");
        assert_eq!(
            fs::read_dir(folder.path().join("assets").join("251108171400"))?.count(),
            1
        );
        Ok(())
    }

    #[test]
    fn without_the_editor_the_image_is_added_to_the_end() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let mut app_state = state_with_diary(folder.path(), EditorMode::Preview);

        let inserted = insert_image(&mut app_state, &picture(outside.path())?)?;

        assert_eq!(inserted, InsertedImage::AppendedToDiary);
        assert!(app_state
            .txt_diary
            .starts_with("Dear diary\n![Güzel Gün](assets/251108171400/"));
        Ok(())
    }

    #[test]
    fn nothing_is_copied_without_an_opened_diary() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let mut app_state = state_with_diary(folder.path(), EditorMode::Edit);
        app_state.current_diary.is_selected = false;

        assert!(insert_image(&mut app_state, &picture(outside.path())?).is_err());
        assert!(!folder.path().join("assets").exists());
        Ok(())
    }

    #[test]
    fn several_images_go_in_one_block_of_paragraphs() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let mut app_state = state_with_diary(folder.path(), EditorMode::Edit);
        let sources = [
            picture_named(outside.path(), "a.png")?,
            picture_named(outside.path(), "b.png")?,
        ];

        let InsertedImage::ForEditor(markdown) = insert_images(&mut app_state, &sources)? else {
            anyhow::bail!("the editor should insert the images");
        };

        let paragraphs = markdown.split("\n\n").collect::<Vec<_>>();
        assert_eq!(paragraphs.len(), 2);
        assert!(paragraphs[0].starts_with("![a](assets/251108171400/a_"));
        assert!(paragraphs[1].starts_with("![b](assets/251108171400/b_"));
        Ok(())
    }

    #[test]
    fn pasted_image_data_is_saved_as_a_screenshot() -> anyhow::Result<()> {
        let folder = tempfile::tempdir()?;
        let mut app_state = state_with_diary(folder.path(), EditorMode::Edit);

        let InsertedImage::ForEditor(markdown) = insert_image_data(&mut app_state, ONE_PIXEL_PNG)?
        else {
            anyhow::bail!("the editor should insert the image");
        };

        assert!(markdown.starts_with("![screenshot](assets/251108171400/screenshot_"));
        assert!(markdown.ends_with(".png)"));
        Ok(())
    }
}
