use mpris::{Event, PlayerFinder};
use rusqlite::{Connection};

const FILE_NAME: &str = "mpressed.db";

#[derive(Debug)]
struct Song {
    artist: String,
    album: String,
    title: String,
    count: u32
}

fn main() {
    let db = Connection::open(FILE_NAME).unwrap();

    &db.execute("CREATE TABLE if not exists song_data (
                        artist TEXT,
                        album TEXT,
                        title TEXT,
                        count INTEGER
                    )", [])
        .expect("Failed to create table");


    write(&db);

    db.close().unwrap();
}

fn write(db: &Connection) {
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
                    let mut song = Song {
                        artist: data.artists().unwrap()[0].to_string(),
                        album: data.album_name().unwrap().to_string(),
                        title: data.title().unwrap().to_string(),
                        count: 1,
                    };

                    let mut statement = db.prepare("SELECT * FROM song_data WHERE artist = (?1) AND album = (?2) AND title = (?3) LIMIT 1").unwrap();
                    let query = statement.query((&song.artist, &song.album, &song.title));

                    let mut is_empty = true;

                    while let Some (row) = query.unwrap().next().unwrap() {
                        is_empty = false;
                        song.count += row.get::<usize, u32>(3).unwrap();
                        break;
                    }

                    if is_empty {
                        db.execute("INSERT INTO song_data (artist, album, title, count) VALUES (?1, ?2, ?3, ?4)",
                                   (&song.artist, &song.album, &song.title, &song.count))
                            .expect(format!("Failed to insert record {:?}", (&song.artist, &song.album, &song.title, &song.count)).as_str());
                    } else {
                        db.execute("UPDATE song_data SET count = count + 1 WHERE artist = (?1) AND album = (?2) AND title = (?3)",
                                   (&song.artist, &song.album, &song.title))
                            .expect(format!("Failed to update record {:?}", (&song.artist, &song.album, &song.title, &song.count)).as_str());
                    }
                },
                _ => (),
            }
            Err(err) => {
                println!("D-Bus error: {}. Aborting.", err);
                break;
            }
        }
    }

    println!("Event stream ended.");
}

