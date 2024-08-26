use mpressed::{SongData, SongDataPlays, FILE_NAME};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEvent};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Constraint, Margin};
use ratatui::widgets::{Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState};
use ratatui::{crossterm::{
    event::{self, KeyCode}
}, Frame, Terminal};
use rusqlite::{Connection};
use std::io;
use std::io::Result;
use std::time::{Duration, Instant};
use ratatui::style::{Stylize};
use ratatui::text::Text;

#[derive(Debug, Default)]
enum Sort {
    Artist,
    Album,
    Title,
    #[default]
    Plays,
}

#[derive(Debug, Default)]
struct TuiState {
    data: Vec<SongDataPlays>,
    sort: Sort,
    table_state: TableState,
    scroll_state: ScrollbarState,
    exit: bool,
}

impl TuiState {
    fn new() -> TuiState {
        TuiState {
            data: vec![],
            sort: Sort::default(),
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::default(),
            exit: false,
        }
    }

    fn update(&mut self) {
        self.data.clear();

        let db: Connection = Connection::open(FILE_NAME).unwrap();

        let mut statement = db.prepare("SELECT artist, album, title, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY song_data.id ORDER BY SUM(plays) DESC").unwrap();
        let rows = statement.query_map((), |row| {
            let song_data = SongData {
                artist: row.get(0)?,
                album: row.get(1)?,
                title: row.get(2)?,
            };

            Ok(SongDataPlays {
                song_data,
                plays: row.get(3)?,
                plays_string: row.get::<usize, u32>(3)?.to_string(),
            })
        }).unwrap();


        rows.into_iter().for_each((|result| {
            self.data.push(result.unwrap());
        }));

        let i = self.data.len();
        let _ = self.scroll_state.content_length(i);
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        self.update();

        let tick_rate = Duration::from_millis(50);
        let mut last_tick = Instant::now();

        while !self.exit {
            // artist totals
            //SELECT artist, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY artist ORDER BY SUM(plays) DESC

            terminal.draw(|frame| self.render_frame(frame))?;
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if event::poll(timeout)? {
                self.handle_events()?;
            }

            // thread::sleep(Duration::from_millis(100));
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        self.render_table(frame);
        // self.render_scrollbar(frame);
    }

    fn render_table(&mut self, frame: &mut Frame) {
        let header = ["Artist", "Album", "Title", "Plays"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .red()
            .bold()
            .height(1);

        let rows = self.data.iter().map(|(data)| {
            let item = [&data.song_data.artist, &data.song_data.album, &data.song_data.title, &data.plays_string];
            item.into_iter()
                .map(|string| Cell::from(Text::from(format!("{string}"))))
                .collect::<Row>()
                .height(1)
        });

        let table = Table::new(rows, Constraint::from_mins([10, 10, 10, 10]))
            .header(header);

        frame.render_stateful_widget(table, frame.area(), &mut self.table_state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .thumb_symbol("▐")
            .end_symbol(Some("↓"));

        frame.render_stateful_widget(
            scrollbar,
            frame.area().inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key_event) = event::read()? {
            self.handle_key_event(key_event);
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            // KeyCode::Char('s') => self.change_sort(),
            KeyCode::Char('q') => self.exit(),
            KeyCode::Up => self.up(),
            KeyCode::Down => self.down(),
            _ => {}
        }
    }

    fn up(&mut self) {
        self.table_state.scroll_up_by(1);
        // self.scroll_state.prev();
    }

    fn down(&mut self) {
        self.table_state.scroll_down_by(1);
        // self.scroll_state.next();
    }

    // fn right() {
    //
    // }
    //
    // fn left() {
    //
    // }

    fn exit(&mut self) {
        self.exit = true;
    }

}

fn main() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // create app and run it
    let mut tui_state = TuiState::new();
    let res = tui_state.run(&mut terminal);

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

