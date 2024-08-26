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
    pub plays: String,
    pub plays_u32: u32,
}

impl SongDataPlays {
    pub const fn ref_array(&self) -> [&String; 4] {
        [&self.song_data.artist, &self.song_data.album, &self.song_data.title, &self.plays]
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
