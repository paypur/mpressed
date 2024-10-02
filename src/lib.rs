use std::fs::create_dir_all;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use dirs::home_dir;

// pub const FILE_NAME: &str = "test.db";
pub const FILE_NAME: &str = "mpressed.db";
pub const MIN_PLAYTIME_MS: i64 = 60000;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SongData {
    pub artist: String,
    pub album: String,
    pub title: String,
}

pub fn get_db_path() -> PathBuf {
    let full_path = home_dir().unwrap().join(PathBuf::from(".config/mpressed"));
    create_dir_all(&full_path).unwrap();
    full_path.join(FILE_NAME)
}

pub fn date_to_unix(mut date: String) -> i64 {
    date.push_str("T00:00:00Z");
    date.parse::<DateTime<Utc>>().unwrap().timestamp()
}