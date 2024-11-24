use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

pub static MAX_DIARY_SUMMARY_LENGTH: usize = 30;

pub fn diary_summary(diary_path: PathBuf) -> anyhow::Result<String> {
    let mut f = BufReader::new(File::open(diary_path)?);

    let mut utf8_buf = Vec::<u8>::new();

    while f.read(&mut utf8_buf) != 0 {
        //
    }

    let result = if diary.chars().count() > 30 {
        let x = diary.chars();
        let x = x.take(30);
        let x: String = x.take(30).collect();

        format!("{}...", x.trim())
    } else {
        diary.trim().to_string()
    };

    result
        .replace("\r", "")
        .replace("\n", " ")
        .trim()
        .to_string()
}
