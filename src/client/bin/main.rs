use mpressed::{SongData, SongDataPlays, FILE_NAME};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEvent};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::prelude::Color;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Text};
use ratatui::widgets::block::Title;
use ratatui::widgets::{Block, BorderType, Cell, List, Padding, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState};
use ratatui::{crossterm::event::{self, KeyCode}, Frame, Terminal};
use rusqlite::Connection;
use std::io::Result;
use std::time::{Duration, Instant};
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
    Artist,
    Album,
    Title,
    #[default]
    Date,
}

#[derive(Debug, Default)]
struct TuiState<'a> {
    data_vec: Vec<SongDataPlays>,
    sorting: Sorting,
    grouping: Grouping,
    header: [&'a str; 4],
    table_state: TableState,
    scroll_state: ScrollbarState,
    exit: bool,
}

impl<'a> TuiState<'a> {
    fn new() -> Self {
        let data_vec = TuiState::get_data();
        let length = data_vec.len() - 1;

        TuiState {
            data_vec,
            sorting: Sorting::default(),
            grouping: Grouping::default(),
            header: ["<Artist>", "<Album>", "<Title>", ">Plays<"],
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(length),
            exit: false,
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {

        let tick_rate = Duration::from_millis(10);
        let mut last_tick = Instant::now();

        while !self.exit {
            // artist totals
            // SELECT artist, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY artist ORDER BY SUM(plays) DESC

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

    fn update_data(&mut self) {
        self.data_vec = TuiState::<'a>::get_data();
        self.resort_data();
    }

    fn resort_data(&mut self) {
        self.data_vec.sort_by(|a, b| {
            match self.sorting {
                Sorting::Artist => a.artist().cmp(b.artist()),
                Sorting::Album => a.album().cmp(b.album()),
                Sorting::Title => a.title().cmp(b.title()),
                // reversed to be descending
                Sorting::Plays => b.plays().cmp(a.plays()),
            }
        });
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        let [main_area, footer_area] = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(3),
        ]).areas(frame.area());

        let [navbar_area, sidebar_area, table_area] = Layout::horizontal([
            Constraint::Fill(2),
            Constraint::Fill(1),
            Constraint::Fill(17)
        ]).areas(main_area);



        self.render_lists(frame, sidebar_area);
        self.render_table(frame, table_area);
        self.render_scrollbar(frame, table_area);

        self.render_footer(frame, footer_area);
    }

    fn render_lists(&self, frame: &mut Frame, area: Rect) {
        let [sort_area, group_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Fill(1)
        ]).areas(area);

        let block = Block::bordered()
            .title(Line::raw(" Sorting ").centered())
            .border_set(border::PLAIN)
            .padding(Padding::uniform(1));

        let sort_list = List::new(["Artist", "Album", "Title", "Plays"])
            .block(block);

        let group_block = Block::bordered()
            .title(Line::raw(" Grouping ").centered())
            .border_set(border::PLAIN)
            .padding(Padding::uniform(1));

        let group_list = List::new(["Artist", "Album", "Title", "Date"])
            .block(group_block);

        frame.render_widget(sort_list, sort_area);
        frame.render_widget(group_list, group_area);
    }

    // https://github.com/ratatui/ratatui/issues/1004

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let rows: Vec<Row> = self.data_vec.iter()
            .map(|data| {
                data.ref_array()
                    .into_iter()
                    .map(|string| Cell::from(Text::from(format!("{string}"))))
                    .collect::<Row>()
                    .height(1)
            })
            .collect();

        let widths = [
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Fill(2),
            Constraint::Max(7)
        ];

        let header = self.header
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .red()
            .bold()
            .height(1);

        // let info = Title::from(Line::from(" (↑/↓) Scroll | (Home/End) Jump | (←/→) Sort | (r) Refresh | (Esc/q) Quit "));

        let block = Block::bordered()
            .title(Line::raw(" Song Table ").centered())
            // .title(title.alignment(Alignment::Center))
            // .title(info.alignment(Alignment::Center).position(Position::Bottom))
            .padding(Padding::new(1, 3, 0, 0))
            .border_set(border::PLAIN);

        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(Color::Red);

        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .highlight_style(selected_style);

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
        let info_footer = Paragraph::new(Line::from(" (↑/↓) Scroll | (Home/End) Jump | (←/→) Sort | (r) Refresh | (Esc/q) Quit "))
            .centered()
            .block(Block::bordered()
                .title(Title::from(" Mpressed ".red().bold())
                    .alignment(Alignment::Center))
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
            KeyCode::Home => self.home(),
            KeyCode::End => self.end(),
            KeyCode::Left => self.sort_prev(),
            KeyCode::Right => self.sort_next(),
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

    fn home(&mut self) {
        self.table_state.select_first();
        self.scroll_state.first();
    }

    fn end(&mut self) {
        self.table_state.select_last();
        self.scroll_state.last();
    }

    fn sort_prev(&mut self) {
        self.header = match self.sorting {
            Sorting::Artist => {
                self.sorting = Sorting::Plays;
                ["<Artist>", "<Album>", "<Title>", ">Plays<"]
            } ,
            Sorting::Album => {
                self.sorting = Sorting::Artist;
                [">Artist<", "<Album>", "<Title>", "<Plays>"]
            } ,
            Sorting::Title => {
                self.sorting = Sorting::Album;
                ["<Artist>", ">Album<", "<Title>", "<Plays>"]
            } ,
            Sorting::Plays => {
                self.sorting = Sorting::Title;
                ["<Artist>", "<Album>", ">Title<", "<Plays>"]
            } ,
        };
        self.resort_data();
    }

    fn sort_next(&mut self) {
        self.header = match self.sorting {
            Sorting::Artist => {
                self.sorting = Sorting::Album;
                ["<Artist>", ">Album<", "<Title>", "<Plays>"]
            } ,
            Sorting::Album => {
                self.sorting = Sorting::Title;
                ["<Artist>", "<Album>", ">Title<", "<Plays>"]
            } ,
            Sorting::Title => {
                self.sorting = Sorting::Plays;
                ["<Artist>", "<Album>", "<Title>", ">Plays<"]
            } ,
            Sorting::Plays => {
                self.sorting = Sorting::Artist;
                [">Artist<", "<Album>", "<Title>", "<Plays>"]
            } ,
        };
        self.resort_data();
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

