use chrono::Local;
use mpris::{Event, PlayerFinder};
use rusqlite::Connection;

const FILE_NAME: &str = "mpressed.db";

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

    &db.execute(
        "CREATE TABLE if not exists song_data (
                        date TEXT,
                        artist TEXT,
                        album TEXT,
                        title TEXT,
                        count INTEGER
                    )",
        [],
    )
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

    println!(
        "Showing event stream for player {}...\n(Exit with Ctrl-C)\n",
        player.identity()
    );

    let events = player.events().expect("Could not start event stream");

    for event in events {
        match event {
            Ok(event) => match event {
                Event::TrackChanged(data) => {
                    let song = Song {
                        date: Local::now().date_naive().to_string(),
                        artist: data.artists().unwrap().join(" / "),
                        album: data.album_name().unwrap().to_string(),
                        title: data.title().unwrap().to_string(),
                        count: 1,
                    };

                    let mut statement = db.prepare("SELECT * FROM song_data WHERE date = (?1) AND artist = (?2) AND album = (?3) AND title = (?4) LIMIT 1").unwrap();
                    let query = statement.query((&song.date, &song.artist, &song.album, &song.title));

                    let mut is_empty = true;

                    while let Some(row) = query.unwrap().next().unwrap() {
                        is_empty = false;
                        // song.count += row.get::<usize, u32>(3).unwrap();
                        break;
                    }

                    if is_empty {
                        db.execute("INSERT INTO song_data (date, artist, album, title, count) VALUES (?1, ?2, ?3, ?4, ?5)",
                                   (&song.date, &song.artist, &song.album, &song.title, &song.count))
                            .expect(format!("Failed to insert record {:?}", (&song.date, &song.artist, &song.album, &song.title, &song.count)).as_str());
                    } else {
                        db.execute("UPDATE song_data SET count = count + 1 WHERE date = (?1) AND artist = (?2) AND album = (?3) AND title = (?4)",
                                   (&song.date, &song.artist, &song.album, &song.title))
                            .expect(format!("Failed to update record {:?}", (&song.date, &song.artist, &song.album, &song.title)).as_str());
                    }
                },
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
