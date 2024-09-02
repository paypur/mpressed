pub const FILE_NAME: &str = "mpressed.db";
pub const MIN_PLAYTIME: i64 = 15;

#[derive(Debug, Default)]
pub struct SongData {
    pub artist: String,
    pub album: String,
    pub title: String,
}

#[derive(Debug, Default)]
pub struct SongDataExtra {
    pub song_data: SongData,
    pub date: String,
    pub plays: String,
    pub plays_u32: u32,
}

impl SongDataExtra {
    pub fn ref_array(&self) -> [&str; 4] {
        [self.artist(), self.album(), self.title(), self.plays()]
    }

    pub fn ref_array_date(&self) -> [&str; 2] {
        [self.date(), self.plays()]
    }

    pub fn ref_array_artist(&self) -> [&str; 2] {
        [self.artist(), self.plays()]
    }

    pub fn ref_array_album(&self) -> [&str; 2] {
        [&self.album(), self.plays()]
    }

    pub fn date(&self) -> &str {
        &self.date
    }

    pub fn artist(&self) -> &str {
        &self.song_data.artist
    }

    pub fn album(&self) -> &str {
        &self.song_data.album
    }

    pub fn title(&self) -> &str {
        &self.song_data.title
    }

    pub fn plays(&self) -> &str {
        &self.plays
    }
}
