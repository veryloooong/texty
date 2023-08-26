use crate::FileType;
use crate::Position;
use crate::Row;
use crate::SearchDirection;
use std::fs;
use std::io::{Error, Write};

#[derive(Default)]
pub struct Document {
    rows: Vec<Row>,
    pub filename: Option<String>,
    is_dirty: bool,
    file_type: FileType,
}

impl Document {
    pub fn open(filename: &str) -> Result<Self, std::io::Error> {
        let contents = fs::read_to_string(filename)?;
        let file_type = FileType::from(filename);
        let mut rows = Vec::new();
        for line in contents.lines() {
            let mut row = Row::from(line);
            row.highlight(file_type.highlighting_options(), None);
            rows.push(row);
        }

        Ok(Self {
            rows,
            filename: Some(filename.to_string()),
            is_dirty: false,
            file_type,
        })
    }

    pub fn save(&mut self) -> Result<(), Error> {
        if let Some(filename) = &self.filename {
            let mut file = fs::File::create(filename)?;
            self.file_type = FileType::from(filename);
            for row in &mut self.rows {
                file.write_all(row.as_bytes())?;
                file.write_all(b"\n")?;
                row.highlight(self.file_type.highlighting_options(), None);
            }
            self.is_dirty = false;
        }

        Ok(())
    }

    pub fn highlight(&mut self, word: Option<&str>) {
        for row in &mut self.rows {
            row.highlight(self.file_type.highlighting_options(), word);
        }
    }

    pub fn insert(&mut self, at: &Position, c: char) {
        self.is_dirty = true;
        if c == '\n' {
            self.insert_newline(at);
            return;
        }
        if at.y >= self.len() {
            let mut row = Row::default();
            row.insert(0, c);
            row.highlight(self.file_type.highlighting_options(), None);
            self.rows.push(row);
        } else {
            let row = self.rows.get_mut(at.y).unwrap();
            row.insert(at.x, c);
            row.highlight(self.file_type.highlighting_options(), None);
        }
    }

    fn insert_newline(&mut self, at: &Position) {
        let len = self.len();
        if at.y > len {
            return;
        }
        if at.y == len {
            self.rows.push(Row::default());
            return;
        }
        let current_row = &mut self.rows[at.y];
        let mut new_row = current_row.split(at.x);
        current_row.highlight(self.file_type.highlighting_options(), None);
        new_row.highlight(self.file_type.highlighting_options(), None);
        self.rows.insert(at.y + 1, new_row);
    }

    pub fn delete(&mut self, at: &Position) {
        let len = self.len();
        if at.y >= len {
            return;
        }

        self.is_dirty = true;

        if at.x == self.rows.get(at.y).unwrap().len() && at.y + 1 < len {
            let next_row = self.rows.remove(at.y + 1);
            let row = self.rows.get_mut(at.y).unwrap();
            row.append(&next_row);
            row.highlight(self.file_type.highlighting_options(), None);
        } else {
            let row = self.rows.get_mut(at.y).unwrap();
            row.delete(at.x);
            row.highlight(self.file_type.highlighting_options(), None);
        }
    }

    pub fn find(&self, query: &str, at: &Position, direction: SearchDirection) -> Option<Position> {
        if at.y >= self.rows.len() {
            return None;
        }
        let mut position = Position { x: at.x, y: at.y };

        let (start, end) = if direction == SearchDirection::Forward {
            (at.y, self.rows.len())
        } else {
            (0, at.y.saturating_add(1))
        };

        for _ in start..end {
            if let Some(row) = self.rows.get(position.y) {
                if let Some(x) = row.find(query, position.x, direction) {
                    position.x = x;
                    return Some(position);
                }
                if direction == SearchDirection::Forward {
                    position.y = position.y.saturating_add(1);
                    position.x = 0;
                } else {
                    position.y = position.y.saturating_sub(1);
                    position.x = self.rows[position.y].len();
                }
            } else {
                return None;
            }
        }

        None
    }

    pub fn row(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }

    pub fn file_type(&self) -> String {
        self.file_type.name()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }
}
