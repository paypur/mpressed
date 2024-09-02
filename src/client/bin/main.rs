use mpressed::{SongData, SongDataExtra, FILE_NAME};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEvent};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::prelude::Color;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Text};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{Block, BorderType, Borders, Cell, List, ListState, Padding, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState};
use ratatui::{crossterm::event::{self, KeyCode}, symbols, Frame, Terminal};
use rusqlite::Connection;
use std::io::Result;
use std::io;

#[derive(Debug, Default)]
enum Sorting {
    Artist,
    Album,
    Title,
    #[default]
    Plays,
}

#[derive(Debug, Default)]
enum Grouping {
    #[default]
    None,
    Date,
    Artist,
    Album,
    // Title,
}

impl Grouping {
    pub fn prev(&mut self){
        *self = match self {
            Grouping::None => Grouping::None,
            Grouping::Date => Grouping::None,
            Grouping::Artist => Grouping::Date,
            Grouping::Album => Grouping::Artist
        }
    }

    pub fn next(&mut self) {
        *self = match self {
            Grouping::None => Grouping::Date,
            Grouping::Date => Grouping::Artist,
            Grouping::Artist => Grouping::Album,
            Grouping::Album => Grouping::Album
        };
    }
}

const HEADER: [&str; 4] = ["[Artist]", "[Album]", "[Title]", "[Plays]"];
const DATE_HEADER: [&str; 2] = ["[Date]", "[Plays]"];
const ARTIST_HEADER: [&str; 2] = ["[Artist]", "[Plays]"];
const ALBUM_HEADER: [&str; 2] = ["[Album]", "[Plays]"];

#[derive(Debug, Default)]
struct TuiState {
    data_vec: Vec<SongDataExtra>,
    sorting: Sorting,
    grouping: Grouping,
    sorting_state: ListState,
    grouping_state: ListState,
    table_state: TableState,
    scroll_state: ScrollbarState,
    exit: bool,
}

const SELECTED_STYLE: Style = Style::new()
    .add_modifier(Modifier::REVERSED)
    .fg(Color::Red);

impl TuiState {
    fn new() -> Self {
        let data_vec = TuiState::get_data();
        let length = data_vec.len() - 1;
        TuiState {
            data_vec,
            sorting: Sorting::default(),
            grouping: Grouping::default(),
            sorting_state: ListState::default().with_selected(Some(0)),
            grouping_state: ListState::default().with_selected(Some(0)),
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(length),
            exit: false,
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {

        // let tick_rate = Duration::from_millis(10);
        // let mut last_tick = Instant::now();

        while !self.exit {

            terminal.draw(|frame| self.render_frame(frame))?;
            // let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            // if event::poll(timeout)? {
            self.handle_events()?;
            // }

            // if last_tick.elapsed() >= tick_rate {
            //     last_tick = Instant::now();
            // }
        }

        Ok(())
    }

    fn get_data() -> Vec<SongDataExtra> {
        let db: Connection = Connection::open(FILE_NAME).unwrap();
        let mut statement = db.prepare("SELECT artist, album, title, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY song_data.id ORDER BY SUM(plays) DESC").unwrap();
        statement.query_map((), |row| {
            let song_data = SongData {
                artist: row.get(0)?,
                album: row.get(1)?,
                title: row.get(2)?,
            };
            Ok(SongDataExtra {
                song_data,
                date: String::new(),
                plays: row.get::<usize, u32>(3)?.to_string(),
                plays_u32: row.get(3)?,
            })
        }).unwrap().map(|r| {r.unwrap()}).collect()
    }

    fn get_data_group_date() -> Vec<SongDataExtra> {
        let db: Connection = Connection::open(FILE_NAME).unwrap();
        let mut statement = db.prepare("SELECT date, SUM(plays) FROM song_plays GROUP BY date ORDER BY SUM(plays) DESC").unwrap();
        statement.query_map((), |row| {
            let song_data = SongData {
                artist: String::new(),
                album: String::new(),
                title: String::new(),
            };
            Ok(SongDataExtra {
                song_data,
                date: row.get(0)?,
                plays: row.get::<usize, u32>(1)?.to_string(),
                plays_u32: row.get(1)?,
            })
        }).unwrap().map(|r| {r.unwrap()}).collect()
    }

    fn get_data_group_artist() -> Vec<SongDataExtra> {
        let db: Connection = Connection::open(FILE_NAME).unwrap();
        let mut statement = db.prepare("SELECT artist, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY artist ORDER BY SUM(plays) DESC").unwrap();
        statement.query_map((), |row| {
            let song_data = SongData {
                artist: row.get(0)?,
                album: String::new(),
                title: String::new(),
            };
            Ok(SongDataExtra {
                song_data,
                date: String::new(),
                plays: row.get::<usize, u32>(1)?.to_string(),
                plays_u32: row.get(1)?,
            })
        }).unwrap().map(|r| {r.unwrap()}).collect()
    }

    fn get_data_group_album() -> Vec<SongDataExtra> {
        let db: Connection = Connection::open(FILE_NAME).unwrap();
        let mut statement = db.prepare("SELECT album, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY album ORDER BY SUM(plays) DESC").unwrap();
        statement.query_map((), |row| {
            let song_data = SongData {
                artist: String::new(),
                album: row.get(0)?,
                title: String::new(),
            };
            Ok(SongDataExtra {
                song_data,
                date: String::new(),
                plays: row.get::<usize, u32>(1)?.to_string(),
                plays_u32: row.get(1)?,
            })
        }).unwrap().map(|r| {r.unwrap()}).collect()
    }

    fn update_data(&mut self) {
        self.data_vec = match self.grouping {
            Grouping::None => TuiState::get_data(),
            Grouping::Date => TuiState::get_data_group_date(),
            Grouping::Artist => TuiState::get_data_group_artist(),
            Grouping::Album => TuiState::get_data_group_album()
        }
    }

    // fn resort_data(&mut self) {
    //     self.data_vec.sort_by(|a, b| {
    //         match self.sorting {
    //             Sorting::Artist => a.artist().cmp(b.artist()),
    //             Sorting::Album => a.album().cmp(b.album()),
    //             Sorting::Title => a.title().cmp(b.title()),
    //             // reversed to be descending
    //             Sorting::Plays => b.plays().cmp(a.plays()),
    //         }
    //     });
    // }
    //
    // fn select_tab(&mut self) {
    //
    // }

    fn render_frame(&mut self, frame: &mut Frame) {
        let [main_area, footer_area] = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(3),
        ]).areas(frame.area());

        let [sidebar_area, table_area] = Layout::horizontal([
            // Constraint::Fill(1),
            Constraint::Length(12),
            Constraint::Fill(9)
        ]).areas(main_area);

        self.render_sidebar(frame, sidebar_area);
        self.render_table(frame, table_area);
        self.render_scrollbar(frame, table_area);

        self.render_footer(frame, footer_area);
    }

    fn render_sidebar(&mut self, frame: &mut Frame, area: Rect) {
        let sort_set = border::Set {
            top_right: symbols::line::NORMAL.horizontal_down,
            bottom_right: symbols::line::NORMAL.vertical_left,
            bottom_left: symbols::line::NORMAL.vertical_right,
            ..border::PLAIN
        };

        let group_set = border::Set {
            bottom_right: symbols::line::NORMAL.horizontal_up,
            ..border::PLAIN
        };

        let [sort_area, group_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Fill(1)
        ]).areas(area);

        let sorting_block = Block::bordered()
            .title(Title::from(" Sorting "))
            .title(Title::from(" Grouping ").position(Position::Bottom))
            .border_set(sort_set)
            // .borders(Borders::TOP | Borders::RIGHT | Borders::LEFT)
            // .border_set(border::PLAIN)
            .padding(Padding::uniform(1));

        let sort_list = List::new(["Plays", "Artist", "Album", "Title"])
            .block(sorting_block)
            .highlight_style(SELECTED_STYLE);

        let group_block = Block::bordered()
            .borders(Borders::LEFT | Borders::BOTTOM | Borders::RIGHT)
            .border_set(group_set)
            .padding(Padding::uniform(1));

        let group_list = List::new(["None", "Date", "Artist", "Album"])
            .block(group_block)
            .highlight_style(SELECTED_STYLE);

        frame.render_stateful_widget(sort_list, sort_area, &mut self.sorting_state);
        frame.render_stateful_widget(group_list, group_area, &mut self.grouping_state);
    }

    // https://github.com/ratatui/ratatui/issues/1004
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(Line::raw(" Song Table ").centered())
            .padding(Padding::new(1, 3, 0, 0))
            .borders(Borders::TOP | Borders::BOTTOM | Borders::RIGHT);

        let table = match self.grouping {
            Grouping::None => {
                let rows: Vec<Row> = self.data_vec.iter()
                    .map(|data| {
                        data.ref_array()
                            .into_iter()
                            .map(|string| Cell::from(Text::from(format!("{string}"))))
                            .collect::<Row>()
                    })
                    .collect();

                let widths = [
                    Constraint::Fill(1),
                    Constraint::Fill(3),
                    Constraint::Fill(3),
                    Constraint::Max(7)
                ];

                let header = HEADER
                    .into_iter()
                    .map(Cell::from)
                    .collect::<Row>()
                    .red()
                    .bold();

                Table::new(rows, widths)
                    .block(block)
                    .header(header)
                    .highlight_style(SELECTED_STYLE)
            },
            Grouping::Date => {
                let rows: Vec<Row> = self.data_vec.iter()
                    .map(|data| {
                        data.ref_array_date()
                            .into_iter()
                            .map(|string| Cell::from(Text::from(format!("{string}"))))
                            .collect::<Row>()
                    })
                    .collect();

                let widths = [
                    Constraint::Fill(1),
                    Constraint::Max(7)
                ];

                let header = DATE_HEADER
                    .into_iter()
                    .map(Cell::from)
                    .collect::<Row>()
                    .red()
                    .bold();

                Table::new(rows, widths)
                    .block(block)
                    .header(header)
                    .highlight_style(SELECTED_STYLE)
            },
            Grouping::Artist => {
                let rows: Vec<Row> = self.data_vec.iter()
                    .map(|data| {
                        data.ref_array_artist()
                            .into_iter()
                            .map(|string| Cell::from(Text::from(format!("{string}"))))
                            .collect::<Row>()
                    })
                    .collect();

                let widths = [
                    Constraint::Fill(1),
                    Constraint::Max(7)
                ];

                let header = ARTIST_HEADER
                    .into_iter()
                    .map(Cell::from)
                    .collect::<Row>()
                    .red()
                    .bold();

                Table::new(rows, widths)
                    .block(block)
                    .header(header)
                    .highlight_style(SELECTED_STYLE)
            },
            Grouping::Album => {
                let rows: Vec<Row> = self.data_vec.iter()
                    .map(|data| {
                        data.ref_array_album()
                            .into_iter()
                            .map(|string| Cell::from(Text::from(format!("{string}"))))
                            .collect::<Row>()
                    })
                    .collect();

                let widths = [
                    Constraint::Fill(1),
                    Constraint::Max(7)
                ];

                let header = ALBUM_HEADER
                    .into_iter()
                    .map(Cell::from)
                    .collect::<Row>()
                    .red()
                    .bold();

                Table::new(rows, widths)
                    .block(block)
                    .header(header)
                    .highlight_style(SELECTED_STYLE)
            }
        };

        frame.render_stateful_widget(table, area, &mut self.table_state);
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
                vertical: 1,
                horizontal: 2,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Line::from(" (↑/↓) Scroll | (Page Up/Down) Jump | (←/→) Grouping | (r) Refresh | (Esc/q) Quit "))
            .centered()
            .block(Block::bordered()
                .title(Title::from(" Mpressed ".red().bold()).alignment(Alignment::Center))
                .border_type(BorderType::Double));

        frame.render_widget(info_footer, area);
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key_event) = event::read()? {
            self.handle_key_event(key_event);
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => self.up(),
            KeyCode::Down => self.down(),
            KeyCode::PageUp => self.start(),
            KeyCode::PageDown => self.end(),
            KeyCode::Left => self.grouping_prev(),
            KeyCode::Right => self.grouping_next(),
            KeyCode::Char('r') => self.update_data(),
            KeyCode::Esc | KeyCode::Char('q') => self.exit(),
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

    fn start(&mut self) {
        self.table_state.select_first();
        self.scroll_state.first();
    }

    fn end(&mut self) {
        self.table_state.select_last();
        self.scroll_state.last();
    }

    fn grouping_prev(&mut self) {
        self.grouping.prev();
        self.grouping_state.select_previous();
        self.table_state.select_first();
        self.update_data();
    }

    fn grouping_next(&mut self) {
        self.grouping.next();
        self.grouping_state.select_next();
        self.table_state.select_first();
        self.update_data();
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

