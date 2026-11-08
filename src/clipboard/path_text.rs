use std::path::{Path, PathBuf};

const FILE_URI_SCHEME: &str = "file://";
const LOCAL_HOST: &str = "localhost";
const HOME_PREFIX: &str = "~/";
const URI_LIST_COMMENT: &str = "#";

/// Returns `text` without one pair of matching double or single quotes around it, as Windows
/// adds them when a file is copied as a path.
fn without_quotes(text: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|quote| {
            text.strip_prefix(*quote)
                .and_then(|rest| rest.strip_suffix(*quote))
        })
        .unwrap_or(text)
}

/// Returns the byte that the two hexadecimal digits at the start of `text` stand for.
fn leading_escaped_byte(text: &str) -> Option<u8> {
    text.get(..2)
        .filter(|digits| digits.bytes().all(|digit| digit.is_ascii_hexdigit()))
        .and_then(|digits| u8::from_str_radix(digits, 16).ok())
}

/// Decodes the `%XX` escapes of a URI, such as `%20` into a space and `%C4%9F` into `ğ`. A `%`
/// that is not followed by two hexadecimal digits stays as it is. Returns `None` when the
/// decoded bytes are not UTF-8.
pub fn percent_decoded(text: &str) -> Option<String> {
    let mut parts = text.split('%');
    let head = parts.next().unwrap_or_default().as_bytes().to_vec();
    let decoded = parts.fold(head, |mut bytes, part| {
        match leading_escaped_byte(part) {
            Some(byte) => {
                bytes.push(byte);
                bytes.extend_from_slice(&part.as_bytes()[2..]);
            }
            None => {
                bytes.push(b'%');
                bytes.extend_from_slice(part.as_bytes());
            }
        }
        bytes
    });
    String::from_utf8(decoded).ok()
}

/// Tells whether `text` is written as an absolute path on Unix (`/home/me`), Windows
/// (`C:\Users` or `C:/Users`) or a Windows network share (`\\server\share`).
fn looks_absolute(text: &str) -> bool {
    let is_drive_path = matches!(
        text.as_bytes(),
        [drive, b':', b'\\' | b'/', ..] if drive.is_ascii_alphabetic()
    );
    text.starts_with('/') || text.starts_with("\\\\") || is_drive_path
}

/// Returns the local path that a `file://` URI points at, such as `/home/me/a b.png` for
/// `file:///home/me/a%20b.png` and `C:/a.png` for `file:///C:/a.png`.
fn path_of_file_uri(uri: &str) -> Option<String> {
    let after_scheme = uri
        .get(..FILE_URI_SCHEME.len())
        .filter(|scheme| scheme.eq_ignore_ascii_case(FILE_URI_SCHEME))
        .and_then(|_| uri.get(FILE_URI_SCHEME.len()..))?;
    let after_host = after_scheme
        .strip_prefix(LOCAL_HOST)
        .unwrap_or(after_scheme);
    let decoded = percent_decoded(after_host)?;

    let windows_drive_path = decoded
        .strip_prefix('/')
        .filter(|rest| looks_absolute(rest) && !rest.starts_with('/'));
    match (windows_drive_path, decoded.starts_with('/')) {
        (Some(drive_path), _) => Some(drive_path.to_string()),
        (None, true) => Some(decoded),
        (None, false) => Some(format!("//{decoded}")),
    }
}

/// Reads one line of copied text as a path: a `file://` URI, a path under the home directory
/// written as `~/...`, or an absolute path, each optionally in quotes.
fn path_of_line(line: &str, home: Option<&Path>) -> Option<PathBuf> {
    let unquoted = without_quotes(line.trim());

    path_of_file_uri(unquoted)
        .map(PathBuf::from)
        .or_else(|| {
            unquoted
                .strip_prefix(HOME_PREFIX)
                .and_then(|relative| home.map(|home| home.join(relative)))
        })
        .or_else(|| looks_absolute(unquoted).then(|| PathBuf::from(unquoted)))
}

/// Reads `text` as a list of paths, one per line, as file managers and `text/uri-list` put
/// them on the clipboard. Empty lines and `#` comments are skipped. Returns `None` unless every
/// other line is a path, so that ordinary text is never taken for a path.
pub fn paths_in_text(text: &str, home: Option<&Path>) -> Option<Vec<PathBuf>> {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(URI_LIST_COMMENT))
        .collect::<Vec<_>>();

    (!lines.is_empty())
        .then_some(lines)?
        .into_iter()
        .map(|line| path_of_line(line, home))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(text: &str) -> Option<Vec<PathBuf>> {
        paths_in_text(text, Some(Path::new("/home/me")))
    }

    #[test]
    fn escapes_are_decoded_into_utf8() {
        assert_eq!(
            percent_decoded("Tatil%20Fotoğraf%C4%B1.png").as_deref(),
            Some("Tatil Fotoğrafı.png")
        );
        assert_eq!(percent_decoded("100%25").as_deref(), Some("100%"));
        assert_eq!(percent_decoded("50% off").as_deref(), Some("50% off"));
        assert_eq!(percent_decoded("bad%FF"), None);
    }

    #[test]
    fn file_uris_become_local_paths() {
        assert_eq!(
            paths("file:///home/me/Deniz%20Kenar%C4%B1.png"),
            Some(vec![PathBuf::from("/home/me/Deniz Kenarı.png")])
        );
        assert_eq!(
            paths("file://localhost/tmp/a.png"),
            Some(vec![PathBuf::from("/tmp/a.png")])
        );
        assert_eq!(
            paths("file:///C:/Users/me/a.png"),
            Some(vec![PathBuf::from("C:/Users/me/a.png")])
        );
        assert_eq!(
            paths("file://server/share/a.png"),
            Some(vec![PathBuf::from("//server/share/a.png")])
        );
    }

    #[test]
    fn a_uri_list_gives_every_file_and_skips_comments() {
        assert_eq!(
            paths("# copied\r\nfile:///a.png\r\nfile:///b.jpg\r\n"),
            Some(vec![PathBuf::from("/a.png"), PathBuf::from("/b.jpg")])
        );
    }

    #[test]
    fn plain_absolute_and_home_paths_are_recognised() {
        assert_eq!(
            paths("  /tmp/a.png  "),
            Some(vec![PathBuf::from("/tmp/a.png")])
        );
        assert_eq!(
            paths("~/Pictures/a.png"),
            Some(vec![PathBuf::from("/home/me/Pictures/a.png")])
        );
        assert_eq!(
            paths(r#""C:\Users\me\a.png""#),
            Some(vec![PathBuf::from(r"C:\Users\me\a.png")])
        );
        assert_eq!(
            paths(r"\\server\share\a.png"),
            Some(vec![PathBuf::from(r"\\server\share\a.png")])
        );
    }

    #[test]
    fn ordinary_text_is_not_taken_for_a_path() {
        assert_eq!(paths("Bugün hava güzeldi."), None);
        assert_eq!(paths("/tmp/a.png\nand some words"), None);
        assert_eq!(paths("https://example.com/a.png"), None);
        assert_eq!(paths("relative/a.png"), None);
        assert_eq!(paths("   \n  "), None);
        assert_eq!(paths_in_text("~/a.png", None), None);
    }
}
