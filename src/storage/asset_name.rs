use std::path::{Component, Path, PathBuf};

use chrono::NaiveDateTime;

pub const ASSETS_DIRECTORY: &str = "assets";
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

const FALLBACK_SLUG: &str = "image";
const TIMESTAMP_FORMAT: &str = "%Y%m%d%H%M%S";

/// Returns the plain ASCII spelling of a lowercase non-ASCII letter, or `None` when it has none.
fn ascii_spelling(lowercase_letter: char) -> Option<&'static str> {
    match lowercase_letter {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => Some("a"),
        'ç' | 'ć' | 'č' => Some("c"),
        'ď' | 'đ' => Some("d"),
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => Some("e"),
        'ğ' => Some("g"),
        'ì' | 'í' | 'î' | 'ï' | 'ı' | 'ī' => Some("i"),
        'ł' => Some("l"),
        'ñ' | 'ń' | 'ň' => Some("n"),
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => Some("o"),
        'ř' => Some("r"),
        'ş' | 'ś' | 'š' => Some("s"),
        'ť' => Some("t"),
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => Some("u"),
        'ý' | 'ÿ' => Some("y"),
        'ź' | 'ż' | 'ž' => Some("z"),
        'ß' => Some("ss"),
        'æ' => Some("ae"),
        'œ' => Some("oe"),
        _ => None,
    }
}

/// Returns `character` spelled with ASCII letters only, or a space when it has no such spelling.
fn transliterate(character: char) -> String {
    let lowercase = character.to_lowercase().next().unwrap_or(character);

    if lowercase.is_ascii() {
        lowercase.to_string()
    } else {
        ascii_spelling(lowercase).unwrap_or(" ").to_string()
    }
}

/// Turns `text` into a lowercase ASCII slug whose words are joined by underscores, such as
/// `"Güzel Gün!"` into `"guzel_gun"`. Returns an empty string when no word is left.
pub fn slugify(text: &str) -> String {
    text.chars()
        .map(transliterate)
        .collect::<String>()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Returns the lowercase extension of `path` when it names an image the app can show.
pub fn image_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|extension| IMAGE_EXTENSIONS.contains(&extension.as_str()))
}

/// Builds the name that a copied image gets: the slug of its original name followed by the
/// moment of copying, such as `beach_photo_20261108171400.jpg`. Returns `None` when `source`
/// is not an image the app can show.
pub fn image_asset_file_name(source: &Path, copied_at: NaiveDateTime) -> Option<String> {
    let extension = image_extension(source)?;
    let slug = source
        .file_stem()
        .map(|stem| slugify(&stem.to_string_lossy()))
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| FALLBACK_SLUG.to_string());

    Some(format!(
        "{slug}_{}.{extension}",
        copied_at.format(TIMESTAMP_FORMAT)
    ))
}

/// Returns `file_name` with `_<number>` put before its extension, or `file_name` itself for the
/// first number, so that several candidates can be tried when a name is already taken.
pub fn numbered_file_name(file_name: &str, number: usize) -> String {
    match (number, file_name.rsplit_once('.')) {
        (0 | 1, _) => file_name.to_string(),
        (_, Some((stem, extension))) => format!("{stem}_{number}.{extension}"),
        (_, None) => format!("{file_name}_{number}"),
    }
}

/// Returns the name of the directory under [`ASSETS_DIRECTORY`] that keeps the assets of the
/// diary `diary_file_name`: the date and time that the diary is named after, such as
/// `251108171400` for `251108171400.md.enc`.
pub fn asset_directory_name(diary_file_name: &str) -> Option<&str> {
    diary_file_name
        .split('.')
        .next()
        .filter(|stem| !stem.is_empty() && slugify(stem) == *stem)
}

/// Builds the markdown reference of an asset, which is relative to the diary folder and always
/// separated by forward slashes.
pub fn asset_reference(directory_name: &str, file_name: &str) -> String {
    format!("{ASSETS_DIRECTORY}/{directory_name}/{file_name}")
}

/// Converts a markdown image reference into a path relative to the diary folder. Returns `None`
/// for web addresses, absolute paths and paths that would leave the diary folder.
pub fn relative_asset_path(reference: &str) -> Option<PathBuf> {
    let is_web_address = reference.contains("://");
    let path = Path::new(reference);
    let stays_inside = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    let names_a_file = path
        .components()
        .any(|component| matches!(component, Component::Normal(_)));

    (!is_web_address && stays_inside && names_a_file).then(|| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn moment() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 8)
            .and_then(|date| date.and_hms_opt(17, 14, 5))
            .unwrap_or_default()
    }

    #[test]
    fn turkish_letters_become_their_ascii_spelling() {
        assert_eq!(
            slugify("Çiğdem Işık Öğütücü Şişe"),
            "cigdem_isik_ogutucu_sise"
        );
        assert_eq!(slugify("İstanbul"), "istanbul");
    }

    #[test]
    fn punctuation_and_spaces_become_single_underscores() {
        assert_eq!(slugify("  My -- Photo (1)!  "), "my_photo_1");
        assert_eq!(slugify("a__b"), "a_b");
    }

    #[test]
    fn letters_without_ascii_spelling_are_dropped() {
        assert_eq!(slugify("日本 trip"), "trip");
        assert_eq!(slugify("日本"), "");
    }

    #[test]
    fn an_image_gets_its_slug_and_the_copy_time() {
        assert_eq!(
            image_asset_file_name(Path::new("/home/me/Tatil Fotoğrafı.JPG"), moment()),
            Some("tatil_fotografi_20261108171405.jpg".to_string())
        );
    }

    #[test]
    fn an_image_without_a_usable_name_is_called_image() {
        assert_eq!(
            image_asset_file_name(Path::new("/tmp/日本.png"), moment()),
            Some("image_20261108171405.png".to_string())
        );
    }

    #[test]
    fn files_that_are_not_images_are_refused() {
        assert_eq!(
            image_asset_file_name(Path::new("/tmp/notes.txt"), moment()),
            None
        );
        assert_eq!(
            image_asset_file_name(Path::new("/tmp/no_extension"), moment()),
            None
        );
    }

    #[test]
    fn numbered_names_keep_the_extension_last() {
        assert_eq!(numbered_file_name("a_1.png", 1), "a_1.png");
        assert_eq!(numbered_file_name("a_1.png", 2), "a_1_2.png");
        assert_eq!(numbered_file_name("plain", 3), "plain_3");
    }

    #[test]
    fn the_asset_directory_is_the_date_and_time_of_the_diary() {
        assert_eq!(
            asset_directory_name("251108171400.md.enc"),
            Some("251108171400")
        );
        assert_eq!(
            asset_directory_name("251108171400.md"),
            Some("251108171400")
        );
        assert_eq!(asset_directory_name(".hidden.md"), None);
        assert_eq!(asset_directory_name("../x.md"), None);
    }

    #[test]
    fn references_use_forward_slashes() {
        assert_eq!(
            asset_reference("251108171400", "a_20261108171405.png"),
            "assets/251108171400/a_20261108171405.png"
        );
    }

    #[test]
    fn only_references_inside_the_folder_are_resolved() {
        assert_eq!(
            relative_asset_path("assets/251108/a.png"),
            Some(PathBuf::from("assets/251108/a.png"))
        );
        assert_eq!(relative_asset_path("../secret.png"), None);
        assert_eq!(relative_asset_path("assets/../../secret.png"), None);
        assert_eq!(relative_asset_path("/etc/passwd"), None);
        assert_eq!(relative_asset_path("https://example.com/a.png"), None);
        assert_eq!(relative_asset_path(""), None);
    }
}
