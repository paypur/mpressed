use mpressed::{SongData, SongDataPlays, FILE_NAME};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEvent};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Alignment, Constraint, Margin, Rect};
use ratatui::widgets::{Block, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState};
use ratatui::{crossterm::{
    event::{self, KeyCode}
}, Frame, Terminal};
use rusqlite::{Connection};
use std::{io};
use std::io::Result;
use std::time::{Duration, Instant};
use ratatui::prelude::Color;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Text};
use ratatui::widgets::block::{Position, Title};

#[derive(Debug, Default)]
enum Sort {
    Artist,
    Album,
    Title,
    #[default]
    Plays,
}

#[derive(Debug, Default)]
struct TuiState<'a> {
    data_vec: Vec<SongDataPlays>,
    sort: Sort,
    table: Table<'a>,
    table_state: TableState,
    scroll_state: ScrollbarState,
    exit: bool,
}

impl<'a> TuiState<'a> {
    fn new() -> Self {
        let data_vec = TuiState::get_data();
        let length = data_vec.len() - 1;

        let rows: Vec<Row> = data_vec.iter()
            .map(|data| {
                data.ref_array()
                    .into_iter()
                    .map(|string| Cell::from(Text::from(format!("{string}"))))
                    .collect::<Row>()
                    .height(1)
            })
            .collect();

        let header = ["[Artist]", "[Album]", "[Title]", "[Plays]"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .red()
            .bold()
            .height(1);

        let title = Title::from(" MPRESSED ".red().bold());
        let info = Title::from(Line::from(" (↑/↓) Up/Down | (q) Quit "));

        let block = Block::bordered()
            .title(title.alignment(Alignment::Center))
            .title(info.alignment(Alignment::Center).position(Position::Bottom))
            .border_set(border::THICK);

        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(Color::Red);

        let table = Table::new(rows, [
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Fill(2),
            Constraint::Max(6)
        ])
            .header(header)
            .block(block)
            .highlight_style(selected_style);

        TuiState {
            data_vec,
            sort: Sort::default(),
            table,
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(length),
            exit: false,
        }
    }

    fn get_data() -> Vec<SongDataPlays> {
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
                plays: row.get::<usize, u32>(3)?.to_string(),
                plays_u32: row.get(3)?,
            })
        }).unwrap();

        rows.map(|r| {r.unwrap()}).collect()
    }

    fn update_data() -> Result<()> {
        todo!()
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {

        let tick_rate = Duration::from_millis(10);
        let mut last_tick = Instant::now();

        while !self.exit {
            // artist totals
            //SELECT artist, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY artist ORDER BY SUM(plays) DESC

            terminal.draw(|frame| self.render_frame(frame))?;
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if event::poll(timeout)? {
                self.handle_events()?;
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        self.render_table(frame, frame.area());
        self.render_scrollbar(frame, frame.area());
    }

    // https://github.com/ratatui/ratatui/issues/1004

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(&self.table, area, &mut self.table_state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .thumb_symbol("█")
            .thumb_style(Color::Red)
            .track_symbol(Some("│"))
            .end_symbol(Some("↓"));

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 2,
                horizontal: 2,
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
            KeyCode::Char('q') => self.exit(),
            KeyCode::Up => self.up(),
            KeyCode::Down => self.down(),
            KeyCode::Left => self.sort_prev(),
            KeyCode::Right => self.sort_next(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn up(&mut self) {
        self.table_state.scroll_up_by(1);
        self.scroll_state.prev();
    }

    fn down(&mut self) {
        self.table_state.scroll_down_by(1);
        self.scroll_state.next();
    }

    fn sort_prev(&mut self) {
        todo!()
    }

    fn sort_next(&self) {
        todo!()
    }

}

fn main() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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

