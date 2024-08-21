use chrono::Local;
use mpris::{Event, PlayerFinder};
use rusqlite::Connection;

const FILE_NAME: &str = "mpressed.db";
const MIN_PLAYTIME: u8 = 10;

#[derive(Debug)]
struct Song {
    date: String,
    artist: String,
    album: String,
    title: String,
    count: u32,
}

fn main() {
    let db = Connection::open(FILE_NAME).unwrap();

    &db.execute("CREATE TABLE if not exists song_data (
                        date TEXT,
                        artist TEXT,
                        album TEXT,
                        title TEXT,
                        count INTEGER
                    )", [])
        .expect("Failed to create table");

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
    let mut song_option: Option<Song> = None;

    for event in player.events().expect("Could not start event stream") {
        match event {
            Ok(event) => match event {
                Event::TrackChanged(data) => {
                    let current_date = Local::now().date_naive().to_string();
                    let current_time = Local::now().timestamp();

                    if current_time - track_last_changed > MIN_PLAYTIME as i64 {
                        match song_option {
                            Some(song) => {
                                match &db.execute("UPDATE song_data SET count = count + 1 WHERE date = (?1) AND artist = (?2) AND album = (?3) AND title = (?4)",
                                                  (&song.date, &song.artist, &song.album, &song.title)) {
                                    Ok(updated) => {
                                        if *updated == 1 {
                                            println!("Updated record: {:?}", (&song.date, &song.artist, &song.album, &song.title));
                                        } else {
                                            match &db.execute("INSERT INTO song_data (date, artist, album, title, count) VALUES (?1, ?2, ?3, ?4, ?5)",
                                                              (&song.date, &song.artist, &song.album, &song.title, &song.count)) {
                                                Ok(_) => println!("Inserted record: {:?}", (&song.date, &song.artist, &song.album, &song.title)),
                                                Err(_) => println!("Failed to insert record: {:?}", (&song.date, &song.artist, &song.album, &song.title)),
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        println!("Failed to update record: {:?}", (&song.date, &song.artist, &song.album, &song.title));
                                    }
                                }
                            }
                            None => (),
                        }
                    } else {
                        let song = song_option.unwrap();
                        println!("Song skipped: {:?}. Minimum playtime ({}s) not met.", (&song.date, &song.artist, &song.album, &song.title), MIN_PLAYTIME);
                    }

                    track_last_changed = current_time;

                    song_option = Some(
                        Song {
                            date: current_date,
                            artist: data.artists().unwrap().join(" / "),
                            album: data.album_name().unwrap().to_string(),
                            title: data.title().unwrap().to_string(),
                            count: 1,
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
