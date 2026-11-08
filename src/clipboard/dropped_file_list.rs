use std::path::PathBuf;

const FILE_LIST_OFFSET_POSITION: usize = 0;
const IS_WIDE_POSITION: usize = 16;

/// Reads the little-endian `u32` at `position` of `bytes`.
fn u32_at(bytes: &[u8], position: usize) -> Option<u32> {
    bytes
        .get(position..position + 4)
        .and_then(|field| field.try_into().ok())
        .map(u32::from_le_bytes)
}

/// Splits a list of names that each end with a zero and that ends with an empty name.
fn names_until_empty<T: PartialEq + Default + Copy>(units: &[T]) -> Vec<&[T]> {
    units
        .split(|unit| *unit == T::default())
        .take_while(|name| !name.is_empty())
        .collect()
}

/// Reads the paths of a Windows `DROPFILES` structure, which Windows puts on the clipboard as
/// `CF_HDROP` when files are copied in Explorer. Returns `None` when the structure is damaged.
pub fn paths_of_dropped_file_list(bytes: &[u8]) -> Option<Vec<PathBuf>> {
    let file_list_offset = usize::try_from(u32_at(bytes, FILE_LIST_OFFSET_POSITION)?).ok()?;
    let is_wide = u32_at(bytes, IS_WIDE_POSITION)? != 0;
    let file_list = bytes.get(file_list_offset..)?;

    if is_wide {
        let units = file_list
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect::<Vec<_>>();
        names_until_empty(&units)
            .into_iter()
            .map(|name| String::from_utf16(name).ok().map(PathBuf::from))
            .collect()
    } else {
        Some(
            names_until_empty(file_list)
                .into_iter()
                .map(|name| PathBuf::from(String::from_utf8_lossy(name).into_owned()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dropfiles_header(is_wide: bool) -> Vec<u8> {
        [
            20u32.to_le_bytes(),
            0u32.to_le_bytes(),
            0u32.to_le_bytes(),
            0u32.to_le_bytes(),
            u32::from(is_wide).to_le_bytes(),
        ]
        .concat()
    }

    #[test]
    fn wide_file_lists_give_every_path() {
        let names = "C:\\Fotoğraf.png\0D:\\b.jpg\0\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let bytes = [dropfiles_header(true), names].concat();

        assert_eq!(
            paths_of_dropped_file_list(&bytes),
            Some(vec![
                PathBuf::from("C:\\Fotoğraf.png"),
                PathBuf::from("D:\\b.jpg")
            ])
        );
    }

    #[test]
    fn narrow_file_lists_are_read_too() {
        let bytes = [dropfiles_header(false), b"C:\\a.png\0\0".to_vec()].concat();

        assert_eq!(
            paths_of_dropped_file_list(&bytes),
            Some(vec![PathBuf::from("C:\\a.png")])
        );
    }

    #[test]
    fn a_damaged_structure_gives_nothing() {
        assert_eq!(paths_of_dropped_file_list(&[1, 2, 3]), None);
        assert_eq!(
            paths_of_dropped_file_list(&[[200u32.to_le_bytes(); 5].concat()].concat()),
            None
        );
    }
}
