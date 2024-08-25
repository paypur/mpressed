use chrono::Local;
use mpris::{Event, PlayerFinder};
use rusqlite::{Connection};

const FILE_NAME: &str = "mpressed.db";
const MIN_PLAYTIME: u8 = 10;

#[derive(Debug)]
struct SongData {
    artist: String,
    album: String,
    title: String,
}

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

    event_loop(&db);

    db.close().unwrap();
}

fn event_loop(db: &Connection) {
    let player = PlayerFinder::new()
        .expect("Could not connect to D-Bus")
        .find_by_name("VLC media player")
        // .find_active()
        .expect("Could not find active player");

    println!("Showing event stream for player {}...\n(Exit with Ctrl-C)\n", player.identity());

    let mut track_last_changed: i64 = 0;
    let mut song_option: Option<SongData> = None;

    for event in player.events().expect("Could not start event stream") {
        match event {
            Ok(event) => match event {
                Event::TrackChanged(data) => {
                    let current_date = Local::now().date_naive().to_string();
                    let current_time = Local::now().timestamp();

                    if current_time - track_last_changed > MIN_PLAYTIME as i64 {
                        match song_option {
                            Some(song) => {
                                match db.execute("INSERT OR IGNORE INTO song_data (artist, album, title) VALUES (?1, ?2, ?3)",
                                                 (&song.artist, &song.album, &song.title)) {
                                    Ok(_) => println!("Inserted song_data: {:?}", (&song.artist, &song.album, &song.title)),
                                    Err(_) => println!("Failed to insert song_data: {:?}", (&song.artist, &song.album, &song.title)),
                                }

                                let mut statement = db.prepare("SELECT ID FROM song_data WHERE artist = (?1) AND album = (?2) AND title = (?3) LIMIT 1")
                                    .unwrap();
                                let mut query = statement.query((&song.artist, &song.album, &song.title))
                                    .unwrap();
                                let row = query.next().unwrap();
                                let id: u32 = row.unwrap().get(0).unwrap();

                                match db.prepare("UPDATE song_plays SET plays = plays + 1 WHERE id = (?1) AND date = (?2)")
                                    .unwrap()
                                    .execute((id, &current_date)) {
                                    Ok(update) => {
                                        if *update == 1 {
                                            println!("Updated song_plays: {:?}", (&song.artist, &song.album, &song.title));
                                        } else {
                                            match db.execute("INSERT INTO song_plays (id, date, plays) VALUES (?1, ?2, ?3)",
                                                              (id, &current_date, 1)) {
                                                Ok(_) => println!("Inserted song_plays: {:?}", (&song.artist, &song.album, &song.title)),
                                                Err(_) => println!("Failed to insert song_plays: {:?}", (&song.artist, &song.album, &song.title)),
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        println!("Failed to update song_plays: {:?}", (&song.artist, &song.album, &song.title));
                                    }
                                }
                            }
                            None => (),
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
                }
                _ => (),
            },
            Err(err) => {
                println!("D-Bus error: {}. Aborting.", err);
                break;
            }
        }
    }

    println!("Event stream ended.");
}
