pub const FILE_NAME: &str = "mpressed.db";
pub const MIN_PLAYTIME: i64 = 1;

#[derive(Debug, Default)]
pub struct SongData {
    pub artist: String,
    pub album: String,
    pub title: String,
}

