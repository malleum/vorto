mod db;

use std::{env, error::Error, io};
use db::{Database, SearchResult, Verse};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap, Clear},
    Frame, Terminal,
};

enum ActivePane {
    Books,
    Chapters,
    Content,
    SearchInput,
    SearchResults,
}

struct App {
    db: Database,
    versions: Vec<String>,
    current_version_idx: usize,
    
    books: Vec<String>,
    books_state: ListState,
    
    chapters: Vec<u32>,
    chapters_state: ListState,
    
    current_verses: Vec<Verse>,
    content_scroll: u16,
    
    active_pane: ActivePane,
    
    search_query: String,
    search_results: Vec<SearchResult>,
    search_results_state: ListState,
    
    show_version_popup: bool,
    versions_state: ListState,
    
    should_quit: bool,
}

impl App {
    fn new(db: Database) -> Self {
        let versions = db.get_versions().unwrap_or_else(|_| vec!["BSB".to_string()]);
        let current_version_idx = versions.iter().position(|v| v == "BSB").unwrap_or(0);
        let version = &versions[current_version_idx];
        
        let books = db.get_books(version).unwrap_or_default();
        let mut books_state = ListState::default();
        if !books.is_empty() { books_state.select(Some(0)); }
        
        let mut app = Self {
            db,
            versions,
            current_version_idx,
            books,
            books_state,
            chapters: vec![],
            chapters_state: ListState::default(),
            current_verses: vec![],
            content_scroll: 0,
            active_pane: ActivePane::Books,
            search_query: String::new(),
            search_results: vec![],
            search_results_state: ListState::default(),
            show_version_popup: false,
            versions_state: ListState::default(),
            should_quit: false,
        };
        
        app.load_chapters();
        app.load_content();
        app
    }

    fn current_version(&self) -> &str {
        &self.versions[self.current_version_idx]
    }

    fn load_chapters(&mut self) {
        if let Some(idx) = self.books_state.selected() {
            if let Some(book) = self.books.get(idx) {
                self.chapters = self.db.get_chapters(self.current_version(), book).unwrap_or_default();
                if !self.chapters.is_empty() {
                    self.chapters_state.select(Some(0));
                } else {
                    self.chapters_state.select(None);
                }
            }
        }
    }

    fn load_content(&mut self) {
        self.content_scroll = 0;
        if let Some(book_idx) = self.books_state.selected() {
            if let Some(book) = self.books.get(book_idx) {
                if let Some(chap_idx) = self.chapters_state.selected() {
                    if let Some(&chap) = self.chapters.get(chap_idx) {
                        self.current_verses = self.db.get_chapter(self.current_version(), book, chap).unwrap_or_default();
                    }
                }
            }
        }
    }

    fn perform_search(&mut self) {
        if self.search_query.is_empty() { return; }
        self.search_results = self.db.search(self.current_version(), &self.search_query).unwrap_or_default();
        if !self.search_results.is_empty() {
            self.search_results_state.select(Some(0));
            self.active_pane = ActivePane::SearchResults;
        } else {
            self.active_pane = ActivePane::Books; // fallback
        }
    }
    
    fn jump_to_search_result(&mut self) {
        if let Some(idx) = self.search_results_state.selected() {
            if let Some(res) = self.search_results.get(idx).cloned() {
                // Find book
                if let Some(b_idx) = self.books.iter().position(|b| b == &res.book) {
                    self.books_state.select(Some(b_idx));
                    self.load_chapters();
                    // Find chapter
                    if let Some(c_idx) = self.chapters.iter().position(|c| c == &res.chapter) {
                        self.chapters_state.select(Some(c_idx));
                        self.load_content();
                        self.active_pane = ActivePane::Content;
                        // Approximate scroll (verse - 1)
                        self.content_scroll = res.verse.saturating_sub(1) as u16 * 2; // rough estimate
                    }
                }
            }
        }
    }
    
    fn switch_version(&mut self, offset: isize) {
        if self.versions.is_empty() { return; }
        let len = self.versions.len() as isize;
        let mut idx = self.current_version_idx as isize + offset;
        idx = (idx % len + len) % len;
        self.current_version_idx = idx as usize;
        
        // Reload all data
        self.books = self.db.get_books(self.current_version()).unwrap_or_default();
        if let Some(sel) = self.books_state.selected() {
            if sel >= self.books.len() {
                self.books_state.select(Some(self.books.len().saturating_sub(1)));
            }
        } else if !self.books.is_empty() {
            self.books_state.select(Some(0));
        }
        self.load_chapters();
        self.load_content();
        
        if !self.search_query.is_empty() && matches!(self.active_pane, ActivePane::SearchResults) {
            self.perform_search();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let db_path = env::var("VORTO_DB_PATH").unwrap_or_else(|_| "bibles.db".to_string());
    
    if !std::path::Path::new(&db_path).exists() {
        eprintln!("Database not found at {}. Run the nix build to generate it.", db_path);
        std::process::exit(1);
    }

    let db = Database::new(&db_path)?;
    let app = App::new(db);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), Box<dyn Error>> where <B as Backend>::Error: 'static {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if app.show_version_popup {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                        app.show_version_popup = false;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.switch_version(1);
                        app.versions_state.select(Some(app.current_version_idx));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.switch_version(-1);
                        app.versions_state.select(Some(app.current_version_idx));
                    }
                    _ => {}
                }
                continue;
            }

            match app.active_pane {
                ActivePane::SearchInput => match key.code {
                    KeyCode::Enter => {
                        app.perform_search();
                    }
                    KeyCode::Esc => {
                        app.active_pane = ActivePane::Books;
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                    }
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                    }
                    _ => {}
                },
                _ => match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Char('v') => {
                        app.show_version_popup = true;
                        app.versions_state.select(Some(app.current_version_idx));
                    }
                    KeyCode::Char('/') => {
                        app.active_pane = ActivePane::SearchInput;
                    }
                    KeyCode::Tab => {
                        app.switch_version(1);
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        app.active_pane = match app.active_pane {
                            ActivePane::Chapters => ActivePane::Books,
                            ActivePane::Content => ActivePane::Chapters,
                            ActivePane::SearchResults => ActivePane::Books,
                            _ => app.active_pane,
                        };
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        app.active_pane = match app.active_pane {
                            ActivePane::Books => ActivePane::Chapters,
                            ActivePane::Chapters => ActivePane::Content,
                            _ => app.active_pane,
                        };
                    }
                    KeyCode::Char('j') | KeyCode::Down => match app.active_pane {
                        ActivePane::Books => {
                            if let Some(i) = app.books_state.selected() {
                                let next = (i + 1).min(app.books.len().saturating_sub(1));
                                app.books_state.select(Some(next));
                                app.load_chapters();
                                app.load_content();
                            }
                        }
                        ActivePane::Chapters => {
                            if let Some(i) = app.chapters_state.selected() {
                                let next = (i + 1).min(app.chapters.len().saturating_sub(1));
                                app.chapters_state.select(Some(next));
                                app.load_content();
                            }
                        }
                        ActivePane::Content => {
                            app.content_scroll = app.content_scroll.saturating_add(1);
                        }
                        ActivePane::SearchResults => {
                            if let Some(i) = app.search_results_state.selected() {
                                let next = (i + 1).min(app.search_results.len().saturating_sub(1));
                                app.search_results_state.select(Some(next));
                            }
                        }
                        _ => {}
                    },
                    KeyCode::Char('k') | KeyCode::Up => match app.active_pane {
                        ActivePane::Books => {
                            if let Some(i) = app.books_state.selected() {
                                let prev = i.saturating_sub(1);
                                app.books_state.select(Some(prev));
                                app.load_chapters();
                                app.load_content();
                            }
                        }
                        ActivePane::Chapters => {
                            if let Some(i) = app.chapters_state.selected() {
                                let prev = i.saturating_sub(1);
                                app.chapters_state.select(Some(prev));
                                app.load_content();
                            }
                        }
                        ActivePane::Content => {
                            app.content_scroll = app.content_scroll.saturating_sub(1);
                        }
                        ActivePane::SearchResults => {
                            if let Some(i) = app.search_results_state.selected() {
                                let prev = i.saturating_sub(1);
                                app.search_results_state.select(Some(prev));
                            }
                        }
                        _ => {}
                    },
                    KeyCode::Enter => {
                        if matches!(app.active_pane, ActivePane::SearchResults) {
                            app.jump_to_search_result();
                        }
                    }
                    _ => {}
                },
            }
        }
        
        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(size);
        
    let top_bar = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" Vorto ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(format!("Version: {} ", app.current_version()), Style::default().fg(Color::Yellow)),
            Span::raw("| (h/j/k/l) Navigate | (v) Change Version | (/) Search | (q) Quit")
        ])
    ]).block(Block::default().borders(Borders::ALL));
    f.render_widget(top_bar, main_layout[0]);
    
    let bottom_area = main_layout[1];

    if matches!(app.active_pane, ActivePane::SearchInput) || matches!(app.active_pane, ActivePane::SearchResults) {
        let search_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(bottom_area);
            
        let input_style = if matches!(app.active_pane, ActivePane::SearchInput) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        
        let search_input = Paragraph::new(app.search_query.as_str())
            .style(input_style)
            .block(Block::default().borders(Borders::ALL).title("Search (Enter to submit, Esc to cancel)"));
        f.render_widget(search_input, search_layout[0]);
        
        let items: Vec<ListItem> = app.search_results.iter().map(|res| {
            let content = Line::from(vec![
                Span::styled(format!("{} {}:{} - ", res.book, res.chapter, res.verse), Style::default().fg(Color::Cyan)),
                Span::raw(&res.text),
            ]);
            ListItem::new(content)
        }).collect();
        
        let border_style = if matches!(app.active_pane, ActivePane::SearchResults) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        
        let results_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Results").border_style(border_style))
            .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
            
        f.render_stateful_widget(results_list, search_layout[1], &mut app.search_results_state);
        
    } else {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(10),
                Constraint::Percentage(70),
            ])
            .split(bottom_area);
            
        let books_items: Vec<ListItem> = app.books.iter().map(|b| ListItem::new(b.as_str())).collect();
        let border_style = if matches!(app.active_pane, ActivePane::Books) { Style::default().fg(Color::Yellow) } else { Style::default() };
        let books_list = List::new(books_items)
            .block(Block::default().borders(Borders::ALL).title("Books").border_style(border_style))
            .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
        f.render_stateful_widget(books_list, panes[0], &mut app.books_state);
        
        let chaps_items: Vec<ListItem> = app.chapters.iter().map(|c| ListItem::new(c.to_string())).collect();
        let border_style = if matches!(app.active_pane, ActivePane::Chapters) { Style::default().fg(Color::Yellow) } else { Style::default() };
        let chaps_list = List::new(chaps_items)
            .block(Block::default().borders(Borders::ALL).title("Chapters").border_style(border_style))
            .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
        f.render_stateful_widget(chaps_list, panes[1], &mut app.chapters_state);
        
        let mut text = vec![];
        for v in &app.current_verses {
            text.push(Line::from(vec![
                Span::styled(format!("{} ", v.verse), Style::default().fg(Color::DarkGray)),
                Span::raw(&v.text),
            ]));
        }
        
        let border_style = if matches!(app.active_pane, ActivePane::Content) { Style::default().fg(Color::Yellow) } else { Style::default() };
        
        let title = if let (Some(b_idx), Some(c_idx)) = (app.books_state.selected(), app.chapters_state.selected()) {
            if let (Some(book), Some(chap)) = (app.books.get(b_idx), app.chapters.get(c_idx)) {
                format!(" {} {} ", book, chap)
            } else { " Content ".to_string() }
        } else { " Content ".to_string() };
        
        let content_p = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style))
            .wrap(Wrap { trim: true })
            .scroll((app.content_scroll, 0));
        f.render_widget(content_p, panes[2]);
    }
    
    if app.show_version_popup {
        let area = centered_rect(40, 40, size);
        f.render_widget(Clear, area);
        
        let items: Vec<ListItem> = app.versions.iter().map(|v| ListItem::new(v.as_str())).collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Select Version (j/k to change, Enter/Esc to close)").border_style(Style::default().fg(Color::Yellow)))
            .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
            
        f.render_stateful_widget(list, area, &mut app.versions_state);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
