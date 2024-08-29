use chrono::Local;
use mpris::{Event, Player, PlayerFinder};
use rusqlite::{Connection};
use mpressed::{SongData, FILE_NAME, MIN_PLAYTIME};

const IDENTITIES: [&str; 1] = [
    "VLC media player"
    // "Brave"
];

fn main() {
    let db = Connection::open(FILE_NAME).unwrap();

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
    let mut player: Player = player_finder.find_active().unwrap();

    loop {
         for identity in IDENTITIES {
            match player_finder.find_by_name(identity) {
                Ok(find) => {
                    player = find;
                    break;
                }
                Err(_) => {}
            }
         }

        println!("Showing event stream for player {}", player.identity());
        event_handler(db, &mut player);
        println!("Event stream ended.");
    }
}

fn event_handler(db: &Connection, player: &mut Player) {
    let mut track_last_changed: i64 = 0;
    let mut song_option: Option<SongData> = None;

    for event_result in player.events().expect("Could not start event stream") {
        if event_result.is_err() {
            println!("D-Bus error: {}. Aborting.", event_result.unwrap_err());
            break;
        }

        match event_result.unwrap() {
            Event::TrackChanged(data) => {
                let current_date = Local::now().date_naive().to_string();
                let current_time = Local::now().timestamp();

                if current_time - track_last_changed > MIN_PLAYTIME as i64 {
                    if song_option.is_some() {
                        let song = song_option.unwrap();

                        db.execute("INSERT OR IGNORE INTO song_data (artist, album, title) VALUES (?1, ?2, ?3)",
                                         (&song.artist, &song.album, &song.title))
                            .expect(&format!("Failed to inserted song_data: {:?}", (&song.artist, &song.album, &song.title)));

                        let mut statement = db.prepare("SELECT ID FROM song_data WHERE artist = (?1) AND album = (?2) AND title = (?3) LIMIT 1").unwrap();
                        let mut query = statement.query((&song.artist, &song.album, &song.title)).unwrap();
                        let row = query.next().unwrap();
                        let id: u32 = row.unwrap().get(0).unwrap();

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
                } else {
                    let song = song_option.unwrap();
                    println!("Skipped song: {:?}, minimum playtime ({}s) not met.", (&song.artist, &song.album, &song.title), MIN_PLAYTIME);
                }

                track_last_changed = current_time;

                song_option = Some(
                    SongData {
                        artist: data.artists().unwrap().join(" / "),
                        album: data.album_name().unwrap().to_string(),
                        title: data.title().unwrap().to_string(),
                    }
                );
            },
            Event::Playing => (),
            _ => (),
        }
    }
}
