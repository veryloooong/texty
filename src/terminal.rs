use std::io::{stdout, Write};

use crossterm::{
    cursor,
    event::{read, Event},
    style::{Color, Colors, ResetColor, SetColors, SetForegroundColor},
    terminal, ExecutableCommand,
};

use crate::Position;

pub struct Size {
    pub width: u16,
    pub height: u16,
}

pub struct Terminal {
    size: Size,
}

impl Terminal {
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Result<Self, std::io::Error> {
        let (width, height) = terminal::size()?;

        terminal::enable_raw_mode()?;

        Ok(Self {
            size: Size {
                width,
                height: height.saturating_sub(2),
            },
        })
    }

    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn clear_screen() {
        stdout()
            .execute(terminal::Clear(terminal::ClearType::All))
            .ok();
    }

    pub fn clear_current_line() {
        stdout()
            .execute(terminal::Clear(terminal::ClearType::CurrentLine))
            .ok();
    }

    pub fn set_colours(colours: Colors) {
        stdout().execute(SetColors(colours)).ok();
    }

    pub fn set_text_colour(colour: Color) {
        stdout().execute(SetForegroundColor(colour)).ok();
    }

    pub fn reset_colours() {
        stdout().execute(ResetColor).ok();
    }

    #[allow(clippy::let_and_return)]
    pub fn read_key() -> Result<Event, std::io::Error> {
        let event = read();
        event
    }

    pub fn flush() -> Result<(), std::io::Error> {
        stdout().flush()
    }

    pub fn position_cursor(position: &Position) {
        let Position { x, y } = position;
        let x = *x as u16;
        let y = *y as u16;
        stdout().execute(cursor::MoveTo(x, y)).ok();
    }

    pub fn quit() {
        Terminal::clear_screen();
        terminal::disable_raw_mode().ok();
        println!("uuuuuuuuuuuuuuuuuuuu ( ;´ - `;)\r");
    }
}
