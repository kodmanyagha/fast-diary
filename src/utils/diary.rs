use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use anyhow::anyhow;

pub static MAX_DIARY_SUMMARY_LENGTH: usize = 30;

pub fn diary_summary(diary_path: PathBuf) -> anyhow::Result<String> {
    let mut file_reader = BufReader::new(File::open(diary_path)?);

    let mut summary_buf = Vec::<u8>::new();
    let mut utf8_buf = Vec::<u8>::new();
    let mut whitespace_received = false;
    while file_reader.read(&mut utf8_buf)? != 0 {
        let character = String::from_utf8(utf8_buf.clone());

        let Ok(character) = character else { continue };
        let Some(character) = character.chars().nth(0) else {
            continue;
        };

        if whitespace_received {
            continue;
        } else {
            summary_buf.extend_from_slice(character.to_string().as_bytes());

            whitespace_received = character.is_whitespace();
        }
    }

    Ok(String::from_utf8(summary_buf)
        .map_err(|_| anyhow!("asd"))?
        .trim()
        .to_string())
}
