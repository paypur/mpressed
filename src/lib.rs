pub const FILE_NAME: &str = "mpressed.db";
pub const MIN_PLAYTIME: u8 = 10;

#[derive(Debug, Default)]
pub struct SongData {
    pub artist: String,
    pub album: String,
    pub title: String,
}

#[derive(Debug, Default)]
pub struct SongDataPlays {
    pub song_data: SongData,
    pub plays: u32,
    pub plays_string: String,
}

impl SongDataPlays {
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
        &self.plays_string
    }
}

impl AsRef<SongData> for SongDataPlays {
    fn as_ref(&self) -> &SongData {
        &self.song_data
    }
}