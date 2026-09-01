mod db;

use arboard::Clipboard;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use db::{Database, SearchResult, Verse};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{env, error::Error, io};

#[derive(Clone)]
enum View {
    Books {
        items: Vec<String>,
        filtered: Vec<usize>,
        state: ListState,
    },
    Chapters {
        book: String,
        items: Vec<u32>,
        filtered: Vec<usize>,
        state: ListState,
    },
    Verses {
        book: String,
        chapter: u32,
        items: Vec<Verse>,
        filtered: Vec<usize>,
        state: ListState,
        visual_start: Option<usize>,
    },
    SearchResults {
        query: String,
        items: Vec<SearchResult>,
        state: ListState,
    },
}

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Filter,
    GlobalSearch,
    JumpMenu,
}

struct App {
    db: Database,
    versions: Vec<String>,
    current_version_idx: usize,
    
    all_books: Vec<String>,
    
    view_stack: Vec<View>,
    jump_list: Vec<Vec<View>>,
    jump_idx: usize,
    
    input_mode: InputMode,
    input_buffer: String,
    
    number_buffer: String,
    jump_menu_buffer: String,
    
    show_version_popup: bool,
    versions_state: ListState,
    
    should_quit: bool,
    
    clipboard: Option<Clipboard>,
    message: Option<String>,
}

impl App {
    fn new(db: Database) -> Self {
        let versions = db.get_versions().unwrap_or_else(|_| vec!["BSB".to_string()]);
        let current_version_idx = versions.iter().position(|v| v == "BSB").unwrap_or(0);
        let version = &versions[current_version_idx];
        
        let books = db.get_books(version).unwrap_or_default();
        let all_books = books.clone();
        
        let filtered = (0..books.len()).collect();
        let mut state = ListState::default();
        if !books.is_empty() {
            state.select(Some(0));
        }
        
        let initial_view = View::Books {
            items: books,
            filtered,
            state,
        };
        
        Self {
            db,
            versions,
            current_version_idx,
            all_books,
            view_stack: vec![initial_view],
            jump_list: Vec::new(),
            jump_idx: 0,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            number_buffer: String::new(),
            jump_menu_buffer: String::new(),
            show_version_popup: false,
            versions_state: ListState::default(),
            should_quit: false,
            clipboard: Clipboard::new().ok(),
            message: None,
        }
    }
    
    fn record_jump(&mut self) {
        self.jump_list.truncate(self.jump_idx);
        self.jump_list.push(self.view_stack.clone());
        self.jump_idx += 1;
    }
    
    fn jump_backward(&mut self) {
        if self.jump_idx > 0 {
            if self.jump_idx == self.jump_list.len() {
                self.jump_list.push(self.view_stack.clone());
            }
            self.jump_idx -= 1;
            self.view_stack = self.jump_list[self.jump_idx].clone();
        }
    }

    fn jump_forward(&mut self) {
        if self.jump_idx + 1 < self.jump_list.len() {
            self.jump_idx += 1;
            self.view_stack = self.jump_list[self.jump_idx].clone();
        }
    }
    
    fn clear_current_filter(&mut self) {
        if let Some(view) = self.view_stack.last_mut() {
            match view {
                View::Books { filtered, items, state, .. } => {
                    let mut orig_idx = None;
                    if let Some(idx) = state.selected() {
                        orig_idx = filtered.get(idx).copied();
                    }
                    *filtered = (0..items.len()).collect();
                    state.select(orig_idx.or(if items.is_empty() { None } else { Some(0) }));
                }
                View::Chapters { filtered, items, state, .. } => {
                    let mut orig_idx = None;
                    if let Some(idx) = state.selected() {
                        orig_idx = filtered.get(idx).copied();
                    }
                    *filtered = (0..items.len()).collect();
                    state.select(orig_idx.or(if items.is_empty() { None } else { Some(0) }));
                }
                View::Verses { filtered, items, state, .. } => {
                    let mut orig_idx = None;
                    if let Some(idx) = state.selected() {
                        orig_idx = filtered.get(idx).copied();
                    }
                    *filtered = (0..items.len()).collect();
                    state.select(orig_idx.or(if items.is_empty() { None } else { Some(0) }));
                }
                _ => {}
            }
        }
    }

    fn current_version(&self) -> &str {
        &self.versions[self.current_version_idx]
    }
    
    fn push_chapters_view(&mut self, book: String) {
        let chapters = self.db.get_chapters(self.current_version(), &book).unwrap_or_default();
        let filtered = (0..chapters.len()).collect();
        let mut state = ListState::default();
        if !chapters.is_empty() {
            state.select(Some(0));
        }
        self.view_stack.push(View::Chapters {
            book,
            items: chapters,
            filtered,
            state,
        });
    }
    
    fn push_verses_view(&mut self, book: String, chapter: u32) {
        let verses = self.db.get_chapter(self.current_version(), &book, chapter).unwrap_or_default();
        let filtered = (0..verses.len()).collect();
        let mut state = ListState::default();
        if !verses.is_empty() {
            state.select(Some(0));
        }
        self.view_stack.push(View::Verses {
            book,
            chapter,
            items: verses,
            filtered,
            state,
            visual_start: None,
        });
    }
    
    fn perform_global_search(&mut self) {
        if self.input_buffer.is_empty() {
            self.input_mode = InputMode::Normal;
            return;
        }
        let results = self.db.search(self.current_version(), &self.input_buffer).unwrap_or_default();
        let mut state = ListState::default();
        if !results.is_empty() {
            state.select(Some(0));
        }
        self.record_jump();
        self.view_stack.push(View::SearchResults {
            query: self.input_buffer.clone(),
            items: results,
            state,
        });
        self.input_buffer.clear();
        self.input_mode = InputMode::Normal;
    }
    
    fn update_filter(&mut self) {
        let query = self.input_buffer.to_lowercase();
        if let Some(view) = self.view_stack.last_mut() {
            match view {
                View::Books { items, filtered, state, .. } => {
                    *filtered = items.iter().enumerate()
                        .filter(|(_, b)| b.to_lowercase().contains(&query))
                        .map(|(i, _)| i)
                        .collect();
                    state.select(if filtered.is_empty() { None } else { Some(0) });
                }
                View::Chapters { items, filtered, state, .. } => {
                    *filtered = items.iter().enumerate()
                        .filter(|(_, c)| c.to_string().contains(&query))
                        .map(|(i, _)| i)
                        .collect();
                    state.select(if filtered.is_empty() { None } else { Some(0) });
                }
                View::Verses { items, filtered, state, .. } => {
                    *filtered = items.iter().enumerate()
                        .filter(|(_, v)| v.text.to_lowercase().contains(&query))
                        .map(|(i, _)| i)
                        .collect();
                    state.select(if filtered.is_empty() { None } else { Some(0) });
                }
                _ => {}
            }
        }
    }
    
    fn process_number_jump(&mut self) {
        if self.number_buffer.is_empty() { return; }
        if let Ok(num) = self.number_buffer.parse::<u32>() {
            let mut jump_info = None;
            
            if let Some(view) = self.view_stack.last_mut() {
                match view {
                    View::Chapters { items, state, filtered, book } => {
                        if let Some(idx) = items.iter().position(|c| *c == num) {
                            if let Some(f_idx) = filtered.iter().position(|&i| i == idx) {
                                state.select(Some(f_idx));
                            }
                            jump_info = Some((book.clone(), num));
                        } else {
                            self.message = Some(format!("Chapter {} not found", num));
                        }
                    }
                    View::Verses { items, state, filtered, .. } => {
                        if let Some(idx) = items.iter().position(|v| v.verse == num) {
                            if let Some(f_idx) = filtered.iter().position(|&i| i == idx) {
                                state.select(Some(f_idx));
                            }
                        } else {
                            self.message = Some(format!("Verse {} not found", num));
                        }
                    }
                    _ => {}
                }
            }
            
            if let Some((book, chap)) = jump_info {
                self.record_jump();
                self.push_verses_view(book, chap);
            }
        }
        self.number_buffer.clear();
    }
    
    fn yank_selection(&mut self) {
        let mut copied_text = String::new();
        let mut success = false;
        let mut count = 0;
        
        if let Some(View::Verses { items, filtered, state, visual_start, book, chapter }) = self.view_stack.last_mut() {
            if let Some(end_idx) = state.selected() {
                let start_idx = visual_start.unwrap_or(end_idx);
                let min_idx = start_idx.min(end_idx);
                let max_idx = start_idx.max(end_idx);
                
                copied_text = format!("{} {}\n", book, chapter);
                for i in min_idx..=max_idx {
                    if let Some(&orig_idx) = filtered.get(i) {
                        if let Some(v) = items.get(orig_idx) {
                            copied_text.push_str(&format!("[{}] {}\n", v.verse, v.text));
                        }
                    }
                }
                count = max_idx - min_idx + 1;
                *visual_start = None;
            }
        }
        
        if count > 0 {
            if let Some(clipboard) = &mut self.clipboard {
                if clipboard.set_text(copied_text.clone()).is_ok() {
                    success = true;
                }
            }
            if !success {
                // Fallback to wl-copy for wayland
                if let Ok(mut child) = std::process::Command::new("wl-copy").stdin(std::process::Stdio::piped()).spawn() {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = std::io::Write::write_all(&mut stdin, copied_text.as_bytes());
                    }
                    if let Ok(status) = child.wait() {
                        if status.success() { success = true; }
                    }
                }
            }
            if !success {
                // Fallback to xclip for X11
                if let Ok(mut child) = std::process::Command::new("xclip").arg("-selection").arg("clipboard").stdin(std::process::Stdio::piped()).spawn() {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = std::io::Write::write_all(&mut stdin, copied_text.as_bytes());
                    }
                    if let Ok(status) = child.wait() {
                        if status.success() { success = true; }
                    }
                }
            }
            
            if success {
                self.message = Some(format!("Yanked {} verse(s)", count));
            } else {
                self.message = Some("Failed to copy (clipboard/wl-copy/xclip failed)".to_string());
            }
        }
    }
    
    fn change_version(&mut self, offset: isize) {
        let len = self.versions.len() as isize;
        let mut idx = self.current_version_idx as isize + offset;
        idx = (idx % len + len) % len;
        self.current_version_idx = idx as usize;
        
        let old_stack = self.view_stack.clone();
        self.view_stack.clear();
        
        let books = self.db.get_books(self.current_version()).unwrap_or_default();
        let filtered = (0..books.len()).collect();
        let mut state = ListState::default();
        if let Some(View::Books { state: old_state, .. }) = old_stack.first() {
            state.select(old_state.selected());
        } else if !books.is_empty() { 
            state.select(Some(0)); 
        }
        
        self.view_stack.push(View::Books { items: books, filtered, state });
        
        for view in old_stack.iter().skip(1) {
            match view {
                View::Chapters { book, state: old_state, .. } => {
                    self.push_chapters_view(book.clone());
                    if let Some(View::Chapters { state, .. }) = self.view_stack.last_mut() {
                        state.select(old_state.selected());
                    }
                }
                View::Verses { book, chapter, state: old_state, .. } => {
                    self.push_verses_view(book.clone(), *chapter);
                    if let Some(View::Verses { state, .. }) = self.view_stack.last_mut() {
                        state.select(old_state.selected());
                    }
                }
                View::SearchResults { query, state: old_state, .. } => {
                    let results = self.db.search(self.current_version(), query).unwrap_or_default();
                    let mut state = ListState::default();
                    state.select(old_state.selected());
                    self.view_stack.push(View::SearchResults {
                        query: query.clone(),
                        items: results,
                        state,
                    });
                }
                _ => {}
            }
        }
    }
}

fn parse_jump_menu(app: &App) -> Option<(String, Option<u32>, Option<u32>)> {
    let parts: Vec<&str> = app.jump_menu_buffer.split_whitespace().collect();
    if parts.is_empty() { return None; }
    
    let mut book_query = parts[0].to_string();
    let mut rest_idx = 1;
    if parts[0].chars().all(char::is_numeric) && parts.len() > 1 && !parts[1].chars().all(char::is_numeric) {
        book_query = format!("{} {}", parts[0], parts[1]);
        rest_idx = 2;
    }
    
    let query_lower = book_query.to_lowercase();
    let mut best_book = None;
    
    for b in &app.all_books {
        if b.to_lowercase() == query_lower {
            best_book = Some(b.clone());
            break;
        }
    }
    
    if best_book.is_none() {
        for b in &app.all_books {
            if b.to_lowercase().starts_with(&query_lower) {
                best_book = Some(b.clone());
                break;
            }
        }
    }
    
    if best_book.is_none() {
        for b in &app.all_books {
            if b.to_lowercase().contains(&query_lower) {
                best_book = Some(b.clone());
                break;
            }
        }
    }
    
    let book = best_book?;
    
    let mut chap = None;
    let mut verse = None;
    
    if rest_idx < parts.len() {
        if let Ok(c) = parts[rest_idx].parse::<u32>() {
            chap = Some(c);
        }
        if rest_idx + 1 < parts.len() {
            if let Ok(v) = parts[rest_idx + 1].parse::<u32>() {
                verse = Some(v);
            }
        }
    }
    
    Some((book, chap, verse))
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), Box<dyn Error>> where <B as Backend>::Error: 'static {
    let mut last_key_was_g = false;
    let mut last_key_was_bracket_left = false;
    let mut last_key_was_bracket_right = false;
    let mut last_key_was_brace_left = false;
    let mut last_key_was_brace_right = false;

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            app.message = None;
            
            if app.show_version_popup {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => app.show_version_popup = false,
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.change_version(1);
                        app.versions_state.select(Some(app.current_version_idx));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.change_version(-1);
                        app.versions_state.select(Some(app.current_version_idx));
                    }
                    _ => {}
                }
                continue;
            }

            match app.input_mode {
                InputMode::Filter => match key.code {
                    KeyCode::Enter => {
                        app.input_mode = InputMode::Normal;
                        app.input_buffer.clear();
                        app.clear_current_filter();
                        enter_view(&mut app);
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.input_buffer.clear();
                        app.clear_current_filter();
                    }
                    KeyCode::Backspace => { app.input_buffer.pop(); app.update_filter(); }
                    KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => move_cursor(&mut app, MoveDir::Up),
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => move_cursor(&mut app, MoveDir::Down),
                    KeyCode::Tab => move_cursor(&mut app, MoveDir::Down),
                    KeyCode::BackTab => move_cursor(&mut app, MoveDir::Up),
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => { app.input_buffer.push(c); app.update_filter(); }
                    _ => {}
                },
                InputMode::GlobalSearch => match key.code {
                    KeyCode::Enter => app.perform_global_search(),
                    KeyCode::Esc => { app.input_buffer.clear(); app.input_mode = InputMode::Normal; }
                    KeyCode::Backspace => { app.input_buffer.pop(); }
                    KeyCode::Char(c) => { app.input_buffer.push(c); }
                    _ => {}
                },
                InputMode::JumpMenu => match key.code {
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.jump_menu_buffer.clear();
                    }
                    KeyCode::Enter => {
                        if let Some((book, chap, verse)) = parse_jump_menu(&app) {
                            app.record_jump();
                            app.view_stack.truncate(1); // root books
                            app.push_chapters_view(book.clone());
                            let c = chap.unwrap_or(1);
                            app.push_verses_view(book.clone(), c);
                            if let Some(v) = verse {
                                if let Some(View::Verses { items, state, .. }) = app.view_stack.last_mut() {
                                    if let Some(idx) = items.iter().position(|x| x.verse == v) {
                                        state.select(Some(idx));
                                    }
                                }
                            }
                            app.input_mode = InputMode::Normal;
                            app.jump_menu_buffer.clear();
                        }
                    }
                    KeyCode::Tab => {
                        if let Some((best_match, _, _)) = parse_jump_menu(&app) {
                            let parts: Vec<&str> = app.jump_menu_buffer.split_whitespace().collect();
                            if parts.len() <= 2 {
                                app.jump_menu_buffer = best_match + " ";
                            }
                        }
                    }
                    KeyCode::Backspace => { app.jump_menu_buffer.pop(); }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => { app.jump_menu_buffer.push(c); }
                    _ => {}
                },
                InputMode::Normal => {
                    let mut handled = false;
                    
                    if let KeyCode::Char('g') = key.code {
                        if last_key_was_g {
                            move_cursor(&mut app, MoveDir::Top);
                            last_key_was_g = false;
                            handled = true;
                        } else {
                            last_key_was_g = true;
                            handled = true;
                        }
                    } else { last_key_was_g = false; }
                    
                    if let KeyCode::Char('[') = key.code {
                        if last_key_was_bracket_left {
                            jump_sibling(&mut app, -1, false);
                            last_key_was_bracket_left = false;
                            handled = true;
                        } else {
                            last_key_was_bracket_left = true;
                            handled = true;
                        }
                    } else { last_key_was_bracket_left = false; }
                    
                    if let KeyCode::Char(']') = key.code {
                        if last_key_was_bracket_right {
                            jump_sibling(&mut app, 1, false);
                            last_key_was_bracket_right = false;
                            handled = true;
                        } else {
                            last_key_was_bracket_right = true;
                            handled = true;
                        }
                    } else { last_key_was_bracket_right = false; }

                    if let KeyCode::Char('{') = key.code {
                        if last_key_was_brace_left {
                            jump_sibling(&mut app, -1, true);
                            last_key_was_brace_left = false;
                            handled = true;
                        } else {
                            last_key_was_brace_left = true;
                            handled = true;
                        }
                    } else { last_key_was_brace_left = false; }
                    
                    if let KeyCode::Char('}') = key.code {
                        if last_key_was_brace_right {
                            jump_sibling(&mut app, 1, true);
                            last_key_was_brace_right = false;
                            handled = true;
                        } else {
                            last_key_was_brace_right = true;
                            handled = true;
                        }
                    } else { last_key_was_brace_right = false; }
                    
                    if handled { continue; }

                    match key.code {
                        KeyCode::Char('q') => app.should_quit = true,
                        KeyCode::Char('-') => {
                            if app.view_stack.len() > 1 {
                                app.record_jump();
                                app.view_stack.pop();
                                app.clear_current_filter();
                            }
                        }
                        KeyCode::Enter => {
                            if !app.number_buffer.is_empty() {
                                app.process_number_jump();
                            } else {
                                app.record_jump();
                                app.clear_current_filter();
                                enter_view(&mut app);
                            }
                        }
                        KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.jump_backward();
                        }
                        KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.jump_forward();
                        }
                        KeyCode::Tab => {
                            app.jump_forward();
                        }
                        KeyCode::Char('j') | KeyCode::Down => move_cursor(&mut app, MoveDir::Down),
                        KeyCode::Char('k') | KeyCode::Up => move_cursor(&mut app, MoveDir::Up),
                        KeyCode::Char('G') => move_cursor(&mut app, MoveDir::Bottom),
                        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => move_cursor(&mut app, MoveDir::PageDown),
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => move_cursor(&mut app, MoveDir::PageUp),
                        KeyCode::Char('t') | KeyCode::Char('c') => {
                            app.show_version_popup = true;
                            app.versions_state.select(Some(app.current_version_idx));
                        }
                        KeyCode::Char('/') => {
                            app.input_mode = InputMode::Filter;
                            app.input_buffer.clear();
                        }
                        KeyCode::Char('S') | KeyCode::Char('?') => {
                            app.input_mode = InputMode::GlobalSearch;
                            app.input_buffer.clear();
                        }
                        KeyCode::Char(' ') => {
                            app.input_mode = InputMode::JumpMenu;
                            app.jump_menu_buffer.clear();
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() => {
                            app.number_buffer.push(c);
                        }
                        KeyCode::Char('v') => {
                            if let Some(View::Verses { state, visual_start, .. }) = app.view_stack.last_mut() {
                                if visual_start.is_none() {
                                    *visual_start = state.selected();
                                    app.message = Some("Visual line selection started".to_string());
                                } else {
                                    *visual_start = None;
                                    app.message = Some("Visual mode cancelled".to_string());
                                }
                            }
                        }
                        KeyCode::Char('y') => {
                            app.yank_selection();
                        }
                        KeyCode::Esc => {
                            app.number_buffer.clear();
                            if let Some(View::Verses { visual_start, .. }) = app.view_stack.last_mut() {
                                *visual_start = None;
                            }
                            app.clear_current_filter();
                        }
                        _ => {}
                    }
                }
            }
        }
        
        if app.should_quit {
            return Ok(());
        }
    }
}

enum MoveDir { Up, Down, Top, Bottom, PageUp, PageDown }

fn move_cursor(app: &mut App, dir: MoveDir) {
    if let Some(view) = app.view_stack.last_mut() {
        let (state, len) = match view {
            View::Books { state, filtered, .. } => (state, filtered.len()),
            View::Chapters { state, filtered, .. } => (state, filtered.len()),
            View::Verses { state, filtered, .. } => (state, filtered.len()),
            View::SearchResults { state, items, .. } => (state, items.len()),
        };
        
        if len == 0 { return; }
        let current = state.selected().unwrap_or(0);
        let next = match dir {
            MoveDir::Up => current.saturating_sub(1),
            MoveDir::Down => (current + 1).min(len - 1),
            MoveDir::Top => 0,
            MoveDir::Bottom => len - 1,
            MoveDir::PageUp => current.saturating_sub(20),
            MoveDir::PageDown => (current + 20).min(len - 1),
        };
        state.select(Some(next));
    }
}

fn enter_view(app: &mut App) {
    let mut new_view = None;
    if let Some(view) = app.view_stack.last() {
        match view {
            View::Books { items, filtered, state, .. } => {
                if let Some(idx) = state.selected() {
                    if let Some(&orig_idx) = filtered.get(idx) {
                        if let Some(book) = items.get(orig_idx) {
                            new_view = Some(View::Chapters {
                                book: book.clone(),
                                items: vec![], filtered: vec![], state: ListState::default(),
                            });
                        }
                    }
                }
            }
            View::Chapters { book, items, filtered, state, .. } => {
                if let Some(idx) = state.selected() {
                    if let Some(&orig_idx) = filtered.get(idx) {
                        if let Some(&chapter) = items.get(orig_idx) {
                            new_view = Some(View::Verses {
                                book: book.clone(),
                                chapter,
                                items: vec![], filtered: vec![], state: ListState::default(), visual_start: None
                            });
                        }
                    }
                }
            }
            View::SearchResults { items, state, .. } => {
                if let Some(idx) = state.selected() {
                    if let Some(res) = items.get(idx) {
                        new_view = Some(View::Verses {
                            book: res.book.clone(),
                            chapter: res.chapter,
                            items: vec![], filtered: vec![], state: ListState::default(), visual_start: None
                        });
                    }
                }
            }
            _ => {}
        }
    }
    
    if let Some(mut view) = new_view {
        match &mut view {
            View::Chapters { book, .. } => {
                app.push_chapters_view(book.clone());
            }
            View::Verses { book, chapter, .. } => {
                let book_name = book.clone();
                let chap = *chapter;
                
                let mut target_verse = None;
                if let Some(View::SearchResults { items, state, .. }) = app.view_stack.last() {
                    if let Some(idx) = state.selected() {
                        if let Some(res) = items.get(idx) {
                            target_verse = Some(res.verse);
                        }
                    }
                }
                
                app.push_verses_view(book_name, chap);
                
                if let Some(tv) = target_verse {
                    if let Some(View::Verses { items, filtered, state, .. }) = app.view_stack.last_mut() {
                        if let Some(v_idx) = items.iter().position(|v| v.verse == tv) {
                            if let Some(f_idx) = filtered.iter().position(|&i| i == v_idx) {
                                state.select(Some(f_idx));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn jump_sibling(app: &mut App, offset: isize, jump_book: bool) {
    let mut current = None;
    for view in app.view_stack.iter().rev() {
        match view {
            View::Verses { book, chapter, .. } => {
                current = Some((book.clone(), *chapter));
                break;
            }
            View::Chapters { book, .. } => {
                current = Some((book.clone(), 1));
                break;
            }
            _ => {}
        }
    }
    
    if let Some((book, chapter)) = current {
        let books = app.db.get_books(app.current_version()).unwrap_or_default();
        if let Some(b_idx) = books.iter().position(|b| b == &book) {
            let mut target_b_idx = b_idx;
            let mut target_chap = chapter;
            
            if jump_book {
                target_b_idx = (b_idx as isize + offset).clamp(0, books.len().saturating_sub(1) as isize) as usize;
                target_chap = 1;
            } else {
                let chapters = app.db.get_chapters(app.current_version(), &book).unwrap_or_default();
                if let Some(c_idx) = chapters.iter().position(|c| c == &chapter) {
                    let new_c_idx = c_idx as isize + offset;
                    if new_c_idx < 0 {
                        target_b_idx = b_idx.saturating_sub(1);
                        let prev_chaps = app.db.get_chapters(app.current_version(), &books[target_b_idx]).unwrap_or_default();
                        target_chap = *prev_chaps.last().unwrap_or(&1);
                    } else if new_c_idx >= chapters.len() as isize {
                        target_b_idx = (b_idx + 1).min(books.len().saturating_sub(1));
                        target_chap = 1;
                    } else {
                        target_chap = chapters[new_c_idx as usize];
                    }
                }
            }
            
            let target_book = books[target_b_idx].clone();
            
            app.record_jump();
            app.view_stack.truncate(1);
            app.push_chapters_view(target_book.clone());
            app.push_verses_view(target_book, target_chap);
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(size);
        
    let path = match app.view_stack.last() {
        Some(View::Books { .. }) => format!("{} / Books", app.current_version()),
        Some(View::Chapters { book, .. }) => format!("{} / {}", app.current_version(), book),
        Some(View::Verses { book, chapter, .. }) => format!("{} / {} / {}", app.current_version(), book, chapter),
        Some(View::SearchResults { query, .. }) => format!("{} / Search: \"{}\"", app.current_version(), query),
        None => "".to_string(),
    };
    
    let mut top_text = vec![Span::styled(format!(" {} ", path), Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))];
    if !app.number_buffer.is_empty() {
        top_text.push(Span::raw(format!(" | Jump: {}", app.number_buffer)));
    }
    
    f.render_widget(Paragraph::new(Line::from(top_text)), main_layout[0]);
    
    if let Some(view) = app.view_stack.last_mut() {
        match view {
            View::Books { items, filtered, state } => {
                let list_items: Vec<ListItem> = filtered.iter().map(|&i| {
                    ListItem::new(format!(" {}", items[i]))
                }).collect();
                let list = List::new(list_items)
                    .highlight_style(Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .highlight_symbol("> ");
                f.render_stateful_widget(list, main_layout[1], state);
            }
            View::Chapters { items, filtered, state, .. } => {
                let list_items: Vec<ListItem> = filtered.iter().map(|&i| {
                    ListItem::new(format!(" Chapter {}", items[i]))
                }).collect();
                let list = List::new(list_items)
                    .highlight_style(Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .highlight_symbol("> ");
                f.render_stateful_widget(list, main_layout[1], state);
            }
            View::Verses { items, filtered, state, visual_start, .. } => {
                let width = main_layout[1].width.saturating_sub(6) as usize;
                let list_items: Vec<ListItem> = filtered.iter().enumerate().map(|(ui_idx, &i)| {
                    let v = &items[i];
                    let mut style = Style::default();
                    
                    if let Some(start) = *visual_start {
                        if let Some(curr) = state.selected() {
                            let min = start.min(curr);
                            let max = start.max(curr);
                            if ui_idx >= min && ui_idx <= max {
                                style = style.bg(Color::Rgb(60, 60, 60));
                            }
                        }
                    }
                    
                    let wrapped = textwrap::wrap(&v.text, width);
                    let mut lines = Vec::new();
                    
                    for (line_idx, line) in wrapped.iter().enumerate() {
                        if line_idx == 0 {
                            lines.push(Line::from(vec![
                                Span::styled(format!("{:>3} ", v.verse), Style::default().fg(Color::DarkGray)),
                                Span::raw(line.to_string()),
                            ]));
                        } else {
                            lines.push(Line::from(vec![
                                Span::raw("    "),
                                Span::raw(line.to_string()),
                            ]));
                        }
                    }
                    
                    ListItem::new(lines).style(style)
                }).collect();
                let list = List::new(list_items)
                    .highlight_style(Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .highlight_symbol(">> ");
                f.render_stateful_widget(list, main_layout[1], state);
            }
            View::SearchResults { items, state, .. } => {
                let width = main_layout[1].width.saturating_sub(6) as usize;
                let list_items: Vec<ListItem> = items.iter().map(|res| {
                    let prefix = format!("{:<15} {:>3}:{:>3} | ", res.book, res.chapter, res.verse);
                    let prefix_len = prefix.len();
                    
                    let wrapped = textwrap::wrap(&res.text, width.saturating_sub(prefix_len));
                    let mut lines = Vec::new();
                    
                    for (line_idx, line) in wrapped.iter().enumerate() {
                        if line_idx == 0 {
                            lines.push(Line::from(vec![
                                Span::styled(prefix.clone(), Style::default().fg(Color::Cyan)),
                                Span::raw(line.to_string()),
                            ]));
                        } else {
                            lines.push(Line::from(vec![
                                Span::raw(" ".repeat(prefix_len)),
                                Span::raw(line.to_string()),
                            ]));
                        }
                    }
                    ListItem::new(lines)
                }).collect();
                let list = List::new(list_items)
                    .highlight_style(Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .highlight_symbol("> ");
                f.render_stateful_widget(list, main_layout[1], state);
            }
        }
    }
    
    let bottom_text = match app.input_mode {
        InputMode::Filter => format!("Filter: {}█", app.input_buffer),
        InputMode::GlobalSearch => format!("Global Search: {}█", app.input_buffer),
        InputMode::JumpMenu => "Jump Menu Active".to_string(),
        InputMode::Normal => {
            if let Some(msg) = &app.message {
                msg.clone()
            } else {
                "q:quit | -:back | Enter:open | t:version | /:filter | Space:jump | S:search | v:select | y:copy | C-o/i:jump-hist".to_string()
            }
        }
    };
    
    f.render_widget(Paragraph::new(bottom_text).style(Style::default().fg(Color::Gray)), main_layout[2]);
    
    if app.show_version_popup {
        let popup_height = (app.versions.len() as u16 + 2).min(size.height);
        let area = top_right_rect(30, popup_height, size);
        f.render_widget(Clear, area);
        
        let items: Vec<ListItem> = app.versions.iter().map(|v| ListItem::new(v.as_str())).collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Version ").border_style(Style::default().fg(Color::Yellow)))
            .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
            
        f.render_stateful_widget(list, area, &mut app.versions_state);
    }
    
    if app.input_mode == InputMode::JumpMenu {
        let area = centered_rect(50, 20, size);
        f.render_widget(Clear, area);
        
        let mut text = vec![
            Line::from(Span::styled("Type: <Book> [Chapter] [Verse]", Style::default().fg(Color::Gray))),
            Line::from(""),
            Line::from(format!("> {}█", app.jump_menu_buffer)),
            Line::from(""),
        ];
        
        if let Some((book, chap, verse)) = parse_jump_menu(&app) {
            let mut preview_line = vec![Span::styled(book.clone(), Style::default().fg(Color::Green))];
            
            let chapters = app.db.get_chapters(app.current_version(), &book).unwrap_or_default();
            let mut c_valid = true;
            if let Some(c) = chap {
                if !chapters.contains(&c) { c_valid = false; }
                preview_line.push(Span::raw(" "));
                preview_line.push(Span::styled(c.to_string(), Style::default().fg(if c_valid { Color::Green } else { Color::Red })));
            }
            
            if let Some(v) = verse {
                let verses = app.db.get_chapter(app.current_version(), &book, chap.unwrap_or(1)).unwrap_or_default();
                let v_valid = c_valid && verses.iter().any(|x| x.verse == v);
                preview_line.push(Span::raw(":"));
                preview_line.push(Span::styled(v.to_string(), Style::default().fg(if v_valid { Color::Green } else { Color::Red })));
            }
            
            text.push(Line::from(preview_line));
        } else {
            text.push(Line::from(Span::styled("No match...", Style::default().fg(Color::DarkGray))));
        }
        
        let block = Block::default().borders(Borders::ALL).title(" Quick Jump ").border_style(Style::default().fg(Color::Cyan));
        f.render_widget(Paragraph::new(text).block(block), area);
    }
}

fn top_right_rect(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(width.min(r.width)),
        ])
        .split(popup_layout[0])[1]
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
