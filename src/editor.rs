use crate::{Document, Row, Terminal};
use crossterm::{
    event::{Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Color, Colors},
};
use std::env;
use std::time::{Duration, Instant};

const STATUS_BG_COLOR: Color = Color::Rgb {
    r: 153,
    g: 217,
    b: 140,
};
const STATUS_FG_COLOR: Color = Color::Rgb {
    r: 43,
    g: 45,
    b: 66,
};

#[derive(Default, Clone, Copy)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

struct StatusMessage {
    message: String,
    time: Instant,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SearchDirection {
    Forward,
    Backward,
}

impl StatusMessage {
    fn from(message: String) -> Self {
        Self {
            message,
            time: Instant::now(),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum TerminalMode {
    Normal,
    Insert,
}

pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    cursor_position: Position,
    offset: Position,
    document: Document,
    status_message: StatusMessage,
    terminal_mode: TerminalMode,
}

impl Editor {
    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut initial_status = String::from("[USAGE] <C-q> = quit | <C-s> = save | <C-f> = find");
        let document = if let Some(filename) = args.get(1) {
            let doc = Document::open(filename);
            if let Ok(doc) = doc {
                doc
            } else {
                initial_status = format!("ERROR: Could not open file: {}", filename);
                Document::default()
            }
        } else {
            Document::default()
        };

        Self {
            should_quit: false,
            terminal: Terminal::default().expect("Cannot initialise terminal."),
            cursor_position: Position::default(),
            offset: Position::default(),
            document,
            status_message: StatusMessage::from(initial_status),
            terminal_mode: TerminalMode::Normal,
        }
    }

    pub fn run(&mut self) {
        Terminal::clear_screen();
        loop {
            if let Err(err) = self.refresh_screen() {
                die(err);
            }
            if self.should_quit {
                break;
            }
            if let Err(err) = self.process_keypress() {
                die(err);
            }
        }
    }

    fn refresh_screen(&mut self) -> Result<(), std::io::Error> {
        Terminal::clear_screen();
        Terminal::position_cursor(&Position::default());
        if self.should_quit {
            Terminal::quit();
        } else {
            self.draw_rows();
            self.draw_status_bar();
            self.draw_message_bar();
            Terminal::position_cursor(&Position {
                x: self.cursor_position.x.saturating_sub(self.offset.x),
                y: self.cursor_position.y.saturating_sub(self.offset.y),
            });
        }
        Terminal::flush()
    }

    fn draw_row(&self, row: &Row) {
        let start = self.offset.x;
        let width = self.terminal.size().width as usize;
        let end = start.saturating_add(width);
        let row = row.render(start, end);
        println!("{row}\r");
    }

    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        for terminal_row in 0..height {
            Terminal::clear_current_line();
            if let Some(row) = self
                .document
                .row(self.offset.y.saturating_add(terminal_row as usize))
            {
                self.draw_row(row);
            } else {
                Terminal::set_text_colour(Color::DarkCyan);
                println!("~\r");
                Terminal::reset_colours();
            }
        }
    }

    fn draw_status_bar(&self) {
        let mut status: String;
        let width = self.terminal.size().width as usize;
        let mut filename = String::from("[unnamed]");

        let modified_state = if self.document.is_dirty() {
            " [modified]"
        } else {
            ""
        };
        
        let current_mode = current_mode(self.terminal_mode);
        
        if let Some(name) = &self.document.filename {
            filename = name.clone();
            filename.truncate(20);
        }
        status = format!(
            "{}:{}:{}{}",
            filename,
            self.cursor_position.y.saturating_add(1),
            self.cursor_position.x.saturating_add(1),
            modified_state
        );

        let file_indicator = format!("{} | {}", self.document.file_type(), current_mode);

        let len = status.len() + file_indicator.len();
        status.push_str(&" ".repeat(width.saturating_sub(len)));
        status = format!("{}{}", status, file_indicator);
        status.truncate(width);

        Terminal::set_colours(Colors::new(STATUS_FG_COLOR, STATUS_BG_COLOR));
        println!("{}\r", status);
        Terminal::reset_colours();
    }

    fn draw_message_bar(&self) {
        Terminal::clear_current_line();
        let message = &self.status_message;
        if Instant::now() - message.time < Duration::new(5, 0) {
            let mut text = message.message.clone();
            text.truncate(self.terminal.size().width as usize);
            print!("{}", text);
        }
    }

    fn save_file(&mut self) {
        if self.document.filename.is_none() {
            let new_name = self.prompt("Save as: ", |_, _, _| {}).unwrap_or(None);
            if new_name.is_none() {
                self.status_message = StatusMessage::from(String::from("Aborted save"));
                return;
            }
            self.document.filename = new_name;
        }

        self.status_message = if self.document.save().is_ok() {
            StatusMessage::from(String::from("Successfully saved file"))
        } else {
            StatusMessage::from(String::from("Failed to save file"))
        };
    }

    fn quit(&mut self) {
        if self.document.is_dirty() {
            let confirmation = self
                .prompt("Confirm to quit without saving? [y/N] ", |_, _, _| {})
                .unwrap_or(None);
            if let Some(s) = confirmation {
                let s = s.to_ascii_lowercase();
                if s == *"y" || s == *"yes" {
                    self.should_quit = true;
                } else {
                    self.status_message = StatusMessage::from(String::from("Aborted quit"));
                }
            } else {
                self.status_message = StatusMessage::from(String::from("Aborted quit"));
            }
        } else {
            self.should_quit = true;
        }
    }

    fn search(&mut self) {
        let current_position = self.cursor_position;
        let mut direction = SearchDirection::Forward;

        let query = self
            .prompt(
                "Search (ESC = cancel, Left | Right = nav): ",
                |editor, key, query| {
                    let mut moved = false;
                    match key.code {
                        KeyCode::Right => {
                            direction = SearchDirection::Forward;
                            moved = true;
                            editor.move_cursor(KeyCode::Right);
                        }
                        KeyCode::Left => {
                            direction = SearchDirection::Backward;
                        }
                        _ => direction = SearchDirection::Forward,
                    }
                    if let Some(position) =
                        editor
                            .document
                            .find(query, &editor.cursor_position, direction)
                    {
                        editor.cursor_position = position;
                        editor.scroll();
                    } else if moved {
                        editor.move_cursor(KeyCode::Left);
                    }

                    editor.document.highlight(Some(query));
                },
            )
            .unwrap_or(None);

        if query.is_none() {
            self.cursor_position = current_position;
            self.scroll();
        }

        self.document.highlight(None);
    }

    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let event = Terminal::read_key()?;

        if let Event::Key(key) = event {
            match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('q')) => self.quit(),
                (KeyModifiers::CONTROL, KeyCode::Char('s')) => self.save_file(),
                (KeyModifiers::CONTROL, KeyCode::Char('f')) => self.search(),
                (_, KeyCode::Char(c)) => {
                    if self.terminal_mode == TerminalMode::Normal {
                        match c {
                            'h' => self.move_cursor(KeyCode::Left),
                            'j' => self.move_cursor(KeyCode::Down),
                            'k' => self.move_cursor(KeyCode::Up),
                            'l' => self.move_cursor(KeyCode::Right),
                            'i' => self.terminal_mode = TerminalMode::Insert,
                            _ => ()
                        }
                    } else {
                        self.document.insert(&self.cursor_position, c);
                        self.move_cursor(KeyCode::Right);
                    }
                }
                (_, KeyCode::Enter) => {
                    if self.terminal_mode == TerminalMode::Insert {
                        self.document.insert(&self.cursor_position, '\n');
                        self.move_cursor(KeyCode::Right);
                    }
                }
                (_, KeyCode::Delete) => {
                    if self.terminal_mode == TerminalMode::Insert {
                        self.document.delete(&self.cursor_position);
                    }
                },
                (_, KeyCode::Backspace) => {
                    if self.terminal_mode == TerminalMode::Insert && (self.cursor_position.x > 0 || self.cursor_position.y > 0) {
                        self.move_cursor(KeyCode::Left);
                        self.document.delete(&self.cursor_position);
                    }
                },
                (_, KeyCode::Esc) => self.terminal_mode = TerminalMode::Normal,
                (_, KeyCode::Up)
                | (_, KeyCode::Down)
                | (_, KeyCode::Left)
                | (_, KeyCode::Right)
                | (_, KeyCode::PageUp)
                | (_, KeyCode::PageDown)
                | (_, KeyCode::Home)
                | (_, KeyCode::End) => self.move_cursor(key.code),
                _ => (),
            }
        }
        self.scroll();

        Ok(())
    }

    fn prompt<C>(&mut self, prompt: &str, mut callback: C) -> Result<Option<String>, std::io::Error>
    where
        C: FnMut(&mut Self, KeyEvent, &String),
    {
        let mut result = String::new();
        loop {
            self.status_message = StatusMessage::from(format!("{}{}", prompt, result));
            self.refresh_screen()?;
            let event = Terminal::read_key()?;

            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Backspace => result.truncate(result.len().saturating_sub(1)),
                    KeyCode::Enter => break,
                    KeyCode::Char(c) => {
                        if !c.is_control() {
                            result.push(c);
                        }
                    }
                    KeyCode::Esc => {
                        result.clear();
                        break;
                    }
                    _ => (),
                }

                callback(self, key, &result);
            }
        }
        self.status_message = StatusMessage::from(String::new());
        if result.is_empty() {
            return Ok(None);
        }
        Ok(Some(result))
    }

    fn scroll(&mut self) {
        let Position { x, y } = self.cursor_position;
        let width = self.terminal.size().width as usize;
        let height = self.terminal.size().height as usize;
        let offset = &mut self.offset;

        if y < offset.y {
            offset.y = y;
        } else if y >= offset.y.saturating_add(height) {
            offset.y = y.saturating_sub(height).saturating_add(1);
        }
        if x < offset.x {
            offset.x = x;
        } else if x >= offset.x.saturating_add(width) {
            offset.x = x.saturating_sub(width).saturating_add(1);
        }
    }

    fn move_cursor(&mut self, key: KeyCode) {
        let Position { mut x, mut y } = self.cursor_position;

        let calculate_width = |y| -> usize {
            if let Some(row) = self.document.row(y) {
                row.len()
            } else {
                0
            }
        };

        let terminal_height = self.terminal.size().height as usize;
        let height = self.document.len();
        let mut width = calculate_width(y);

        match key {
            KeyCode::Up => y = y.saturating_sub(1),
            KeyCode::Down => {
                if y < height {
                    y = y.saturating_add(1);
                }
            }
            KeyCode::Left => {
                if x > 0 {
                    x -= 1;
                } else if y > 0 {
                    y -= 1;
                    x = calculate_width(y);
                }
            }
            KeyCode::Right => {
                if x < width {
                    x += 1;
                } else {
                    if y < height {
                        y = y.saturating_add(1);
                    };
                    x = 0;
                }
            }
            KeyCode::PageUp => y = y.saturating_sub(terminal_height),
            KeyCode::PageDown => y = usize::min(y.saturating_add(terminal_height), height),
            KeyCode::Home => {
                if x == 0 {
                    y = y.saturating_sub(1);
                }
                x = 0;
            }
            KeyCode::End => {
                if x == width && y < height {
                    y = y.saturating_add(1);
                }
                width = calculate_width(y);
                x = width;
            }
            _ => (),
        }
        width = calculate_width(y);

        x = usize::min(x, width);
        self.cursor_position = Position { x, y };
    }
}

fn die(e: std::io::Error) {
    crossterm::terminal::disable_raw_mode().ok();
    Terminal::clear_screen();
    panic!("{}", e);
}

fn current_mode(mode: TerminalMode) -> String {
    match mode {
        TerminalMode::Normal => String::from("Normal"),
        TerminalMode::Insert => String::from("Insert")
    }
}