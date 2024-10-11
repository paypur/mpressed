use log::info;
use std::collections::{HashMap, HashSet};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::prelude::Color;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Text, ToSpan};
use ratatui::widgets::block::Title;
use ratatui::widgets::{Axis, BarChart, Block, BorderType, Cell, Chart, Dataset, GraphType, LegendPosition, LineGauge, List, ListState, Padding, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState};
use ratatui::{crossterm::event::{self, KeyCode}, symbols, Frame, Terminal};
use rusqlite::Connection;
use std::io;
use std::io::Result;
use chrono::{DateTime, Utc};
use strum::Display;
use mpressed::{date_to_unix, get_db_path};

#[derive(Clone, Debug, Default)]
struct SongDataNone {
    artist: String,
    album: String,
    title: String,
    plays_string: String,
    plays: u32
}

impl SongDataNone {
    pub fn new(artist: String, album: String, title: String, plays: u32) -> Self {
        Self {
            artist,
            album,
            title,
            plays_string: plays.to_string(),
            plays,
        }
    }

    pub fn ref_array(&self) -> [&str; 4] {
        [&self.artist, &self.album, &self.title, &self.plays_string]
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn album(&self) -> &str {
        &self.album
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn plays_string(&self) -> &str {
        &self.plays_string
    }

    pub fn plays(&self) -> u32 {
        self.plays
    }
}

#[derive(Clone, Debug, Default)]
struct SongDataDate {
    date: String,
    plays_string: String,
    plays: u32
}

impl SongDataDate {
    pub fn new(date: String, plays: u32) -> Self {
        Self {
            date,
            plays_string: plays.to_string(),
            plays,
        }
    }

    pub fn ref_array(&self) -> [&str; 2] {
        [&self.date, &self.plays_string]
    }
}

#[derive(Debug, Default)]
struct SongDataArtist {
    artist: String,
    plays_string: String,
    plays: u32
}

impl SongDataArtist {
    pub fn new(artist: String, plays: u32) -> Self {
        Self {
            artist,
            plays_string: plays.to_string(),
            plays,
        }
    }

    pub fn ref_array(&self) -> [&str; 2] {
        [&self.artist, &self.plays_string]
    }
}

#[derive(Debug, Default)]
struct SongDataAlbum {
    album: String,
    plays_string: String,
    plays: u32
}

impl SongDataAlbum {
    pub fn new(album: String, plays: u32) -> Self {
        Self {
            album,
            plays_string: plays.to_string(),
            plays
        }
    }

    pub fn ref_array(&self) -> [&str; 2] {
        [&self.album, &self.plays_string]
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
enum SelectedTab {
    #[default]
    Table,
    Sort,
    Group,
    Filter
}

#[derive(Debug, Default, Display)]
enum Sort {
    Artist,
    Album,
    Title,
    #[default]
    Plays,
}

#[derive(Debug, Default)]
struct SortDirection(Sort, bool);

#[derive(Debug, Default)]
enum Group {
    #[default]
    None,
    Date,
    Artist,
    Album,
}

impl SelectedTab {
    pub fn prev(&mut self) {
        *self = match self {
            SelectedTab::Table => SelectedTab::Filter,
            SelectedTab::Group => SelectedTab::Table,
            SelectedTab::Sort => SelectedTab::Group,
            SelectedTab::Filter => SelectedTab::Sort,
        }
    }

    pub fn next(&mut self) {
        *self = match self {
            SelectedTab::Table => SelectedTab::Group,
            SelectedTab::Group => SelectedTab::Sort,
            SelectedTab::Sort => SelectedTab::Filter,
            SelectedTab::Filter => SelectedTab::Table,
        }
    }
}

impl Group {
    pub fn prev(&mut self) {
        *self = match self {
            Group::None => Group::None,
            Group::Date => Group::None,
            Group::Artist => Group::Date,
            Group::Album => Group::Artist
        }
    }

    pub fn next(&mut self) {
        *self = match self {
            Group::None => Group::Date,
            Group::Date => Group::Artist,
            Group::Artist => Group::Album,
            Group::Album => Group::Album
        };
    }
}

struct TuiState {
    data_vec_none: Vec<SongDataNone>,
    data_vec_date: Vec<SongDataDate>,
    data_vec_artist: Vec<SongDataArtist>,
    data_vec_album: Vec<SongDataAlbum>,
    sort_priority: Vec<SortDirection>,
    group: Group,
    selected_tab: SelectedTab,
    group_state: ListState,
    sort_state: ListState,
    table_state: TableState,
    scroll_state: ScrollbarState,
    filter_artist: HashMap<String, bool>,
    exit: bool,
}

const SELECTED_STYLE: Style = Style::new()
    .add_modifier(Modifier::REVERSED)
    .fg(Color::Red);

impl TuiState {
    fn new() -> Self {
        let data_vec_none = TuiState::get_data_vec_none();
        let data_vec_date = TuiState::get_data_vec_date();
        let data_vec_artist = TuiState::get_data_vec_artist();
        let data_vec_album = TuiState::get_data_vec_album();

        let length = data_vec_none.len();

        TuiState {
            data_vec_none,
            data_vec_date,
            data_vec_artist,
            data_vec_album,
            selected_tab: SelectedTab::default(),
            sort_priority: vec!(SortDirection(Sort::Title, false), SortDirection(Sort::Album, false), SortDirection(Sort::Artist, false), SortDirection(Sort::Plays, true)),
            group: Group::default(),
            sort_state: ListState::default().with_selected(Some(0)),
            group_state: ListState::default().with_selected(Some(0)),
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(length),
            // menu_state: MenuState::<String>::new(vec![
            //     MenuItem::item("Foo", "label_foo".to_string()),
            //     MenuItem::group("Group", vec![
            //         MenuItem::item("Bar 1", "label_bar_1".to_string()),
            //         MenuItem::item("Bar 2", "label_bar_1".to_string()),
            //     ])
            // ]),
            filter_artist: TuiState::get_filter_artist(),
            exit: false,
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // let tick_rate = Duration::from_millis(10);
        // let mut last_tick = Instant::now();

        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            // let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            // avoids waiting for event blocking thread
            // if event::poll(timeout)? {
            self.handle_events()?;
            // }

            // if last_tick.elapsed() >= tick_rate {
            //     last_tick = Instant::now();
            // }
        }

        Ok(())
    }

    fn get_data_vec_none() -> Vec<SongDataNone> {
        Connection::open(get_db_path())
            .unwrap()
            .prepare("SELECT artist, album, title, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY song_data.id ORDER BY SUM(plays) DESC")
            .unwrap()
            .query_map((), |row| Ok(SongDataNone::new(row.get(0)?, row.get(1)?, row.get(2)?, row.get::<usize, u32>(3)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    fn get_data_vec_date() -> Vec<SongDataDate> {
        Connection::open(get_db_path())
            .unwrap()
            .prepare("SELECT date, SUM(plays) FROM song_plays GROUP BY date ORDER BY SUM(plays) DESC")
            .unwrap()
            .query_map((), |row| Ok(SongDataDate::new(row.get(0)?, row.get::<usize, u32>(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    fn get_data_vec_artist() -> Vec<SongDataArtist> {
        Connection::open(get_db_path())
            .unwrap()
            .prepare("SELECT artist, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY artist ORDER BY SUM(plays) DESC")
            .unwrap()
            .query_map((), |row| Ok(SongDataArtist::new(row.get(0)?, row.get::<usize, u32>(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    fn get_data_vec_album() -> Vec<SongDataAlbum> {
        Connection::open(get_db_path())
            .unwrap()
            .prepare("SELECT album, SUM(plays) FROM song_data JOIN song_plays ON song_data.id = song_plays.id GROUP BY album ORDER BY SUM(plays) DESC")
            .unwrap()
            .query_map((), |row| Ok(SongDataAlbum::new(row.get(0)?, row.get::<usize, u32>(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    fn sort_data(&mut self) {
        for sort_direction in &self.sort_priority {
            // TODO: change
            self.data_vec_none.sort_by(|a, b| {
                let order = match sort_direction.0  {
                    Sort::Artist => a.artist().cmp(b.artist()),
                    Sort::Album => a.album().cmp(b.album()),
                    Sort::Title => a.title().cmp(b.title()),
                    Sort::Plays => a.plays().cmp(&b.plays()),
                };
                if sort_direction.1 { order.reverse() } else { order }
            })
        }
    }

    fn update_data(&mut self) {
        match self.group {
            Group::None => self.data_vec_none = TuiState::get_data_vec_none(),
            Group::Date => self.data_vec_date = TuiState::get_data_vec_date(),
            Group::Artist => self.data_vec_artist = TuiState::get_data_vec_artist(),
            Group::Album => self.data_vec_album = TuiState::get_data_vec_album(),
        };
        self.sort_data();
        self.table_state.select_first();
        self.scroll_reset();
    }

    fn get_filter_artist() -> HashMap<String, bool> {
        Connection::open(get_db_path())
            .unwrap()
            .prepare("SELECT artist FROM song_data ORDER BY artist DESC")
            .unwrap()
            .query_map((), |row| Ok((row.get(0)?, true)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect::<HashMap<String, bool>>()
    }

    fn update_filter_artist(&mut self) {

    }

    fn filter(&mut self) {
        // add some sort off filter pop up
        // artist: list all artists?, regex?
        // date: week, month, year, user range


        // self.data_vec_none = self.data_vec_none.iter()
        //     // crashes
        //     .filter(|row| row.artist.clone() <  "2024-09-01".to_string())
        //     .cloned()
        //     .collect();



        self.data_vec_none = self.data_vec_none.iter()
            .filter(|row| self.filter_artist.get(row.artist()).is_some_and(|b| *b))
            .cloned()
            .collect::<Vec<SongDataNone>>();
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        let [main_area, footer_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
        ]).areas(frame.area());

        let [sidebar_area, table_area] = Layout::horizontal([
            Constraint::Length(15),
            Constraint::Fill(1)
        ]).areas(main_area);

        match self.group {
            // Group::None => {}
            Group::Date => {
                let [table_area_small, chart_area] = Layout::horizontal([
                    Constraint::Fill(1),
                    Constraint::Fill(2)
                ]).areas(table_area);

                self.render_sidebar(frame, sidebar_area);
                self.render_table(frame, table_area_small);
                self.render_line_chart_date(frame, chart_area);
                self.render_footer(frame, footer_area);
            }
            // Group::Artist => {}
            // Group::Album => {}
            _ => {
                self.render_sidebar(frame, sidebar_area);
                self.render_table(frame, table_area);
                self.render_footer(frame, footer_area);
            }
        }

        // let menu = Menu::new();
        // frame.render_stateful_widget(menu, menu_area, &mut self.menu_state);

    }

    fn render_sidebar(&mut self, frame: &mut Frame, area: Rect) {
        let [group_area, sort_area, filter_area] = Layout::vertical([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Fill(1)
        ]).areas(area);

        let group_border_style = match self.selected_tab {
            SelectedTab::Group => Style::from(Color::Red),
            _ => Style::default(),
        };

        let group_block = Block::bordered()
            .title(Title::from(" Group ").alignment(Alignment::Center))
            .border_style(group_border_style)
            .padding(Padding::horizontal(1));

        let group_list = List::new(["None", "Date", "Artist", "Album"])
            .block(group_block)
            .highlight_symbol("> ")
            .highlight_style(SELECTED_STYLE);

        frame.render_stateful_widget(group_list, group_area, &mut self.group_state);

        let sort_border_style = match self.selected_tab {
            SelectedTab::Sort => Style::from(Color::Red),
            _ => Style::default(),
        };

        let sort_block = Block::bordered()
            .title(Title::from(" Sort ").alignment(Alignment::Center))
            .border_style(sort_border_style)
            .padding(Padding::horizontal(1));

        let sort_vector = self.sort_priority.iter()
            .rev()
            .enumerate()
            .map(|(i, sort)| format!("{}. {} {}", i+1, if sort.1 { "⌃" } else { "⌄" }, sort.0.to_string()))
            .collect::<Vec<String>>();

        let sort_list = List::new(sort_vector)
            .block(sort_block)
            .highlight_style(SELECTED_STYLE);

        frame.render_stateful_widget(sort_list, sort_area, &mut self.sort_state);

        let filter_block = Block::bordered()
            .title(Title::from(" Filter ").alignment(Alignment::Center))
            .border_style(if self.selected_tab == SelectedTab::Filter { Style::from(Color::Red) } else { Style::default() })
            .padding(Padding::horizontal(1));

        let filter_list = List::new(self.filter_artist.keys().cloned().collect::<Vec<String>>())
            .block(filter_block)
            .highlight_style(SELECTED_STYLE);

        frame.render_widget(filter_list, filter_area);
    }

    // https://github.com/ratatui/ratatui/issues/1004
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let border_style = match self.selected_tab {
            SelectedTab::Table => Style::from(Color::Red),
            _ => Style::default(),
        };

        let block = Block::bordered()
            .title(Line::raw(" Song Table ").centered())
            .border_style(border_style)
            .padding(Padding::new(1, 3, 0, 0));

        let header = match self.group {
            Group::None => vec!["[Artist]", "[Album]", "[Title]", "[Plays]"],
            Group::Date => vec!["[Date]", "[Plays]"],
            Group::Artist => vec!["[Artist]", "[Plays]"],
            Group::Album => vec!["[Album]", "[Plays]"]
        }.into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .red()
            .height(2);

        let widths = match self.group {
            Group::None => vec![Constraint::Fill(1), Constraint::Fill(3), Constraint::Fill(3), Constraint::Max(7)],
            _ => vec![Constraint::Fill(1), Constraint::Max(7)]
        };

        let rows: Vec<Row> = match self.group {
            Group::None => {
                self.data_vec_none.iter()
                    .map(|data| {
                        data.ref_array()
                            .into_iter()
                            .map(|string| Cell::from(Text::from(string)))
                            .collect::<Row>()
                    })
                    .collect()
            },
            Group::Date => {
                self.data_vec_date.iter()
                    .map(|data| {
                        data.ref_array()
                            .into_iter()
                            .map(|string| Cell::from(Text::from(string)))
                            .collect::<Row>()
                    })
                    .collect()
            },
            Group::Artist => {
                self.data_vec_artist.iter()
                    .map(|data| {
                        data.ref_array()
                            .into_iter()
                            .map(|string| Cell::from(Text::from(string)))
                            .collect::<Row>()
                    })
                    .collect()
            },
            Group::Album => {
                self.data_vec_album.iter()
                    .map(|data| {
                        data.ref_array()
                            .into_iter()
                            .map(|string| Cell::from(Text::from(string)))
                            .collect::<Row>()
                    })
                    .collect()
            }
        };

        let table = Table::new(rows, widths)
            .block(block)
            .header(header)
            .highlight_style(SELECTED_STYLE);

        frame.render_stateful_widget(table, area, &mut self.table_state);

        let line = LineGauge::default()
            .style(Style::from(Color::default()))
            .label("")
            .ratio(1.);

        let mut small_area = area.clone();
        small_area.height = 1;
        // TODO: crashes
        small_area.width -= 5; // 5
        small_area.x += 1;
        small_area.y += 2;

        frame.render_widget(line, small_area);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .thumb_symbol("█")
            .thumb_style(Color::Red)
            .track_symbol(Some("│"))
            .end_symbol(Some("↓"));

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {vertical: 1, horizontal: 2}),
            &mut self.scroll_state,
        );
    }

    fn render_line_chart_date(&self, frame: &mut Frame, area: Rect) {
        let mut cloned = self.data_vec_date.clone();
        cloned.sort_by(|a, b| a.date.cmp(&b.date));

        let min_date = cloned[0].date.clone();
        let max_date = cloned[cloned.len() - 1].date.clone();

        let data = cloned.iter()
            .map(|song| {
                (date_to_unix(song.date.clone()) as f64, song.plays as f64)
            })
            .collect::<Vec<(f64, f64)>>();

        let min_time = data[0].0;
        let max_time = data[data.len() - 1].0;

        let max_plays = data.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap().1;

        let dataset = vec![
            Dataset::default()
                .marker(symbols::Marker::Braille)
                .style(Style::default())
                .graph_type(GraphType::Line)
                .data(&data)
        ];

        let chart = Chart::new(dataset)
            .block(
                Block::bordered()
                    .title(
                        Title::default()
                            .content(" Line chart ")
                            .alignment(Alignment::Center),
                    )
                    .padding(Padding::uniform(1))
            )
            .x_axis(
                Axis::default()
                    .title("Date")
                    .style(Style::default())
                    .bounds([min_time, max_time])
                    .labels([min_date, max_date]),
            )
            .y_axis(
                Axis::default()
                    .title("Plays")
                    .style(Style::default())
                    .bounds([0.0, max_plays])
                    .labels(["0".bold(), max_plays.to_span()]),
            )
            .legend_position(Some(LegendPosition::TopLeft))
            .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));

        frame.render_widget(chart, area);
    }

    fn render_bar_chart(&self, frame: &mut Frame, area: Rect) {
        // let block = Block::bordered()
        //     .title(
        //         Title::default()
        //             .content(" Bar chart ")
        //             .alignment(Alignment::Center),
        //     )
        //     .padding(Padding::uniform(1));

        let data = self.data_vec_artist.iter()
            .map(|song| (song.artist.as_str(), song.plays as u64))
            .collect::<Vec<(&str, u64)>>();

        let bar_chart = BarChart::default()
            // .block(block)
            .bar_width(1)
            .bar_gap(0)
            // .bar_style(Style::from(Color::Red))
            .direction(Direction::Horizontal)
            .data(&data);

        frame.render_widget(bar_chart, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Line::from("(Esc/q) Quit | (Tab) Change Tab | (↑/↓) Scroll | (Page Up/Down) Jump | (r) Refresh"))
            .centered()
            .block(
                Block::bordered()
                    .title(Title::from(" Mpressed ".red().bold()).alignment(Alignment::Center))
                    .border_type(BorderType::Double)
            );

        frame.render_widget(info_footer, area);
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key_event) = event::read()? {
            match key_event.code {
                KeyCode::BackTab => self.selected_tab_prev(),
                KeyCode::Tab => self.selected_tab_next(),
                KeyCode::Char('r') => self.update_data(),
                KeyCode::Esc | KeyCode::Char('q') => self.exit(),
                _ => {}
            }
            match self.selected_tab {
                SelectedTab::Table => {
                    match key_event.code {
                        KeyCode::Up => self.table_up(),
                        KeyCode::Down => self.table_down(),
                        KeyCode::PageUp => self.table_start(),
                        KeyCode::PageDown => self.table_end(),
                        _ => {}
                    }
                }
                SelectedTab::Sort => {
                    match key_event.code {
                        KeyCode::Up => self.sort_prev(),
                        KeyCode::Down => self.sort_next(),
                        KeyCode::Enter => self.sort_select(),
                        KeyCode::Char('s') => self.sort_reverse(),
                        _ => {}
                    }
                }
                SelectedTab::Group => {
                    match key_event.code {
                        KeyCode::Up => self.group_prev(),
                        KeyCode::Down => self.group_next(),
                        _ => {}
                    }
                }
                SelectedTab::Filter => {
                    match key_event.code {
                        KeyCode::Enter => self.filter(),
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn selected_tab_prev(&mut self) {
        self.selected_tab.prev();
    }

    fn selected_tab_next(&mut self) {
        self.selected_tab.next();
    }

    fn table_up(&mut self) {
        self.table_state.scroll_up_by(1);
        self.scroll_state.prev();
    }

    fn table_down(&mut self) {
        self.table_state.scroll_down_by(1);
        self.scroll_state.next();
    }

    fn table_start(&mut self) {
        self.table_state.select_first();
        self.scroll_state.first();
    }

    fn table_end(&mut self) {
        self.table_state.select_last();
        self.scroll_state.last();
    }

    fn sort_prev(&mut self) {
        self.sort_state.select_previous();
    }

    fn sort_next(&mut self) {
        self.sort_state.select_next();
    }

    fn sort_select(&mut self) {
        // subtract because sorting_priority is reversed compared to sorting_state
        let s = self.sort_priority.len() - self.sort_state.selected().unwrap() - 1;
        let temp: SortDirection = self.sort_priority.remove(s);
        self.sort_priority.push(temp);
        self.sort_data();
    }

    fn sort_reverse(&mut self) {
        // subtract because sorting_priority is reversed compared to sorting_state
        let s = self.sort_priority.len() - self.sort_state.selected().unwrap() - 1;
        self.sort_priority[s].1 = !self.sort_priority[s].1;
        self.sort_data();
    }

    fn group_prev(&mut self) {
        self.group.prev();
        self.group_state.select_previous();
        self.table_state.select_first();
        self.scroll_reset();
    }

    fn group_next(&mut self) {
        self.group.next();
        self.group_state.select_next();
        self.table_state.select_first();
        self.scroll_reset();
    }

    fn scroll_reset(&mut self) {
        self.scroll_state = ScrollbarState::new(
            match self.group {
                Group::None => self.data_vec_none.len(),
                Group::Date => self.data_vec_date.len(),
                Group::Artist => self.data_vec_artist.len(),
                Group::Album => self.data_vec_album.len(),
            }
        );
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

