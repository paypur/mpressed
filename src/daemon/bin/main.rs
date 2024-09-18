use std::thread::sleep;
use std::time::Duration;
use chrono::Local;
use log::{debug};
use mpris::{Metadata, PlaybackStatus, Player, PlayerFinder};
use rusqlite::{Connection};
use mpressed::{get_db_path, SongData, MIN_PLAYTIME_MS};

const IDENTITIES: [&str; 1] = [
    "VLC media player"
    // "Brave"
];

fn main() {
    env_logger::init();

    let db = Connection::open(get_db_path()).unwrap();

    db.execute("CREATE TABLE if not exists song_data (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                artist TEXT,
                album TEXT,
                title TEXT,
                UNIQUE(artist, album, title)
            )", [])
        .expect("Failed to create song_data table");

    db.execute("CREATE TABLE if not exists song_plays (
                id INTEGER,
                date TEXT,
                plays INTEGER,
                UNIQUE(id, date)
            )", [])
        .expect("Failed to create song_plays table");

    player_loop(&db);
}

fn player_loop(db: &Connection) {
    let player_finder: PlayerFinder = PlayerFinder::new().expect("Could not connect to D-Bus");

    loop {
        for identity in IDENTITIES {
            if player_finder.find_by_name(identity).is_ok() {
                println!("Showing event stream for player {}", identity);
                tracker_loop(db, &mut player_finder.find_by_name(identity).unwrap());
                println!("Event stream ended.");
                break;
            }
        }

        sleep(Duration::from_secs(1));
    }
}

fn tracker_loop(db: &Connection, player: &mut Player) {
    let mut written = false;
    let mut song_option: Option<SongData> = get_song_data(&player.get_metadata().unwrap());
    let mut song_playtime: i64 = 0;
    let mut current_date = Local::now().date_naive().to_string();

    let mut player_tracker = player.track_progress(1000)
        .expect("Failed to start progress tracker");

    loop {
        debug!("tick: {}, {:?}", song_playtime, song_option);

        let last_tick = Local::now().timestamp_millis();
        let tick = player_tracker.tick();

        if player.get_playback_status().unwrap() != PlaybackStatus::Playing {
            continue;
        }

        let song_current = get_song_data(tick.progress.metadata());

        if song_option.is_some() && song_current.is_some() && song_current.clone().unwrap() == song_option.clone().unwrap() {
            song_playtime += Local::now().timestamp_millis() - last_tick;
            if !written && song_playtime >= MIN_PLAYTIME_MS {
                write(db, &song_current.unwrap(), &current_date);
                written = true;
            }
            continue;
        }

        written = false;
        song_option = song_current;
        song_playtime = 0;
        current_date = Local::now().date_naive().to_string();
    }
}

fn get_song_data(data: &Metadata) -> Option<SongData> {
    Some(SongData {
        // ISSUE
        // opus only allows for one artist and joins by ","
        // other formats join by " / "
        artist: data.artists().unwrap().join(" / "),
        album: data.album_name().unwrap().to_string(),
        title: data.title().unwrap().to_string(),
    })
}

fn write(db: &Connection, song: &SongData, current_date: &str) {
    if *song == SongData::default() {
        return;
    }

    db.execute("INSERT OR IGNORE INTO song_data (artist, album, title) VALUES (?1, ?2, ?3)",
               (&song.artist, &song.album, &song.title))
        .expect(&format!("Failed to inserted song_data: {:?}", (&song.artist, &song.album, &song.title)));

    let id: u32 = db.prepare("SELECT ID FROM song_data WHERE artist = (?1) AND album = (?2) AND title = (?3) LIMIT 1")
        .unwrap()
        .query((&song.artist, &song.album, &song.title))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .get(0)
        .unwrap();

    let update = db.prepare("UPDATE song_plays SET plays = plays + 1 WHERE id = (?1) AND date = (?2)")
        .unwrap()
        .execute((id, &current_date))
        .expect(&format!("Failed to update song_plays: {:?}", (&song.artist, &song.album, &song.title)));

    if update as u32 == 1 {
        println!("Updated song_plays: {:?}", (&song.artist, &song.album, &song.title));
    } else {
        match db.execute("INSERT INTO song_plays (id, date, plays) VALUES (?1, ?2, ?3)",
                         (id, &current_date, 1)) {
            Ok(_) => println!("Inserted song_plays: {:?}", (&song.artist, &song.album, &song.title)),
            Err(_) => println!("Failed to insert song_plays: {:?}", (&song.artist, &song.album, &song.title)),
        }
    }
}
