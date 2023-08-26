use crossterm::style::Color;

#[derive(PartialEq, Clone, Copy)]
pub enum Type {
    None,
    Number,
    String,
    Character,
    Comment,
    PrimaryKeywords,
    SecondaryKeywords,
    Match,
}

impl Type {
    pub fn to_colour(self) -> Color {
        match self {
            Type::Number => Color::Rgb {
                r: 244,
                g: 162,
                b: 97,
            },
            Type::String => Color::Rgb {
                r: 233,
                g: 237,
                b: 201,
            },
            Type::Character => Color::Rgb {
                r: 255,
                g: 200,
                b: 221,
            },
            Type::Comment => Color::Rgb {
                r: 133,
                g: 153,
                b: 0,
            },
            Type::PrimaryKeywords => Color::Green,
            Type::SecondaryKeywords => Color::Yellow,
            Type::Match => Color::Cyan,
            Type::None => Color::White,
        }
    }
}
