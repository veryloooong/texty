use crate::{highlighting, HighlightingOptions, SearchDirection};
use crossterm::style::{Color, SetForegroundColor};
use std::cmp;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default)]
pub struct Row {
    content: String,
    highlighting: Vec<highlighting::Type>,
    len: usize,
}

impl From<&str> for Row {
    fn from(slice: &str) -> Self {
        Self {
            content: String::from(slice),
            highlighting: Vec::new(),
            len: slice[..].graphemes(true).count(),
        }
    }
}

impl Row {
    pub fn render(&self, start: usize, end: usize) -> String {
        let end = cmp::min(end, self.content.len());
        let start = cmp::min(start, end);
        let mut rendered = String::new();
        let mut current_highlighting = &highlighting::Type::None;

        for (i, grapheme) in self.content[..]
            .graphemes(true)
            .enumerate()
            .skip(start)
            .take(end - start)
        {
            if let Some(c) = grapheme.chars().next() {
                let highlighting_type = self
                    .highlighting
                    .get(i)
                    .unwrap_or(&highlighting::Type::None);
                if highlighting_type != current_highlighting {
                    current_highlighting = highlighting_type;
                    rendered.push_str(
                        format!("{}", SetForegroundColor(highlighting_type.to_colour())).as_str(),
                    );
                }

                if c == '\t' {
                    rendered.push(' ');
                } else {
                    rendered.push(c);
                }
            }
        }

        rendered.push_str(format!("{}", SetForegroundColor(Color::Reset)).as_str());

        rendered
    }

    pub fn insert(&mut self, at: usize, c: char) {
        if at >= self.len() {
            self.content.push(c);
            self.len += 1;
        } else {
            let mut result = String::new();
            let mut length = 0;

            for (i, grapheme) in self.content[..].graphemes(true).enumerate() {
                length += 1;
                if i == at {
                    length += 1;
                    result.push(c);
                }
                result.push_str(grapheme);
            }

            self.len = length;
            self.content = result;
        }
    }

    pub fn delete(&mut self, at: usize) {
        if at < self.len() {
            let mut result = String::new();
            let mut length = 0;

            for (i, grapheme) in self.content[..].graphemes(true).enumerate() {
                if i == at {
                    continue;
                }
                length += 1;
                result.push_str(grapheme);
            }

            self.len = length;
            self.content = result;
        }
    }

    pub fn append(&mut self, new: &Self) {
        self.content = format!("{}{}", self.content, new.content);
        self.len += new.len;
    }

    pub fn split(&mut self, at: usize) -> Self {
        let mut first_row = String::new();
        let mut second_row = String::new();
        let mut first_len = 0;
        let mut second_len = 0;

        for (i, grapheme) in self.content[..].graphemes(true).enumerate() {
            if i < at {
                first_len += 1;
                first_row.push_str(grapheme);
            } else {
                second_len += 1;
                second_row.push_str(grapheme);
            }
        }

        self.content = first_row;
        self.len = first_len;

        Self {
            content: second_row,
            highlighting: Vec::new(),
            len: second_len,
        }
    }

    pub fn find(&self, query: &str, at: usize, direction: SearchDirection) -> Option<usize> {
        if at > self.len || query.is_empty() {
            return None;
        }

        let (start, end) = if direction == SearchDirection::Forward {
            (at, self.len)
        } else {
            (0, at)
        };

        let substring = self.content[..]
            .graphemes(true)
            .skip(start)
            .take(end - start)
            .collect::<String>();
        let matching_index = if direction == SearchDirection::Forward {
            substring.find(query)
        } else {
            substring.rfind(query)
        };

        if let Some(matching_index) = matching_index {
            for (grapheme_index, (byte_index, _)) in
                self.content[..].grapheme_indices(true).enumerate()
            {
                if matching_index == byte_index {
                    return Some(start.saturating_add(grapheme_index));
                }
            }
        }

        None
    }

    pub fn highlight(&mut self, opts: &HighlightingOptions, word: Option<&str>) {
        self.highlighting = Vec::new();

        let chars = self.content.chars().collect::<Vec<char>>();

        let mut index = 0;
        while let Some(c) = chars.get(index) {
            if self.highlight_numbers(&mut index, opts, *c, &chars)
                || self.highlight_strings(&mut index, opts, *c, &chars)
                || self.highlight_char(&mut index, opts, *c, &chars)
                || self.highlight_comments(&mut index, opts, *c, &chars)
                || self.highlight_primary_keywords(&mut index, opts, &chars)
                || self.highlight_secondary_keywords(&mut index, opts, &chars)
            {
                continue;
            }
            self.highlighting.push(highlighting::Type::None);
            index += 1;
        }

        self.highlight_matches(word);
    }

    fn highlight_matches(&mut self, word: Option<&str>) {
        if let Some(word) = word {
            if word.is_empty() {
                return;
            }
            let mut search_index = 0;

            while let Some(search_match) = self.find(word, search_index, SearchDirection::Forward) {
                if let Some(next_index) = search_match.checked_add(word[..].graphemes(true).count())
                {
                    for i in search_index.saturating_add(search_match)..next_index {
                        self.highlighting[i] = highlighting::Type::Match;
                    }
                    search_index = next_index;
                } else {
                    break;
                }
            }
        }
    }

    fn highlight_strings(
        &mut self,
        index: &mut usize,
        opts: &HighlightingOptions,
        c: char,
        chars: &[char],
    ) -> bool {
        if opts.strings() && c == '"' {
            loop {
                self.highlighting.push(highlighting::Type::String);
                *index += 1;
                if let Some(next_char) = chars.get(*index) {
                    if next_char == &'"' {
                        break;
                    }
                } else {
                    break;
                }
            }
            self.highlighting.push(highlighting::Type::String);
            *index += 1;
            return true;
        }

        false
    }

    fn highlight_numbers(
        &mut self,
        index: &mut usize,
        opts: &HighlightingOptions,
        c: char,
        chars: &[char],
    ) -> bool {
        if opts.numbers() && c.is_ascii_digit() {
            if *index > 0 {
                let prev_char = chars[*index - 1];
                if !is_separator(prev_char) {
                    return false;
                }
            }

            loop {
                self.highlighting.push(highlighting::Type::Number);
                *index += 1;
                if let Some(next_char) = chars.get(*index) {
                    if next_char != &'.'
                        && next_char != &'e'
                        && next_char != &'_'
                        && !next_char.is_ascii_digit()
                    {
                        break;
                    }
                } else {
                    break;
                }
            }

            return true;
        }

        false
    }

    fn highlight_char(
        &mut self,
        index: &mut usize,
        opts: &HighlightingOptions,
        c: char,
        chars: &[char],
    ) -> bool {
        if opts.characters() && c == '\'' {
            if let Some(next_char) = chars.get(index.saturating_add(1)) {
                let closing_index = if next_char == &'\\' {
                    index.saturating_add(3)
                } else {
                    index.saturating_add(2)
                };
                if let Some(closing_char) = chars.get(closing_index) {
                    if closing_char == &'\'' {
                        for _ in 0..=closing_index.saturating_sub(*index) {
                            self.highlighting.push(highlighting::Type::Character);
                            *index += 1;
                        }
                        return true;
                    }
                }
            }
        }

        false
    }

    fn highlight_comments(
        &mut self,
        index: &mut usize,
        opts: &HighlightingOptions,
        c: char,
        chars: &[char],
    ) -> bool {
        if opts.comments() && c == '/' && *index < chars.len() {
            if let Some(next_char) = chars.get(index.saturating_add(1)) {
                if next_char == &'/' {
                    for _ in *index..chars.len() {
                        self.highlighting.push(highlighting::Type::Comment);
                        *index += 1;
                    }
                    return true;
                }
            }
        }

        false
    }

    fn highlight_substring(
        &mut self,
        index: &mut usize,
        substring: &str,
        chars: &[char],
        hl_type: highlighting::Type,
    ) -> bool {
        if substring.is_empty() {
            return false;
        }
        for (i, c) in substring.chars().enumerate() {
            if let Some(next_char) = chars.get(index.saturating_add(i)) {
                if next_char != &c {
                    return false;
                }
            } else {
                return false;
            }
        }
        for _ in 0..substring.len() {
            self.highlighting.push(hl_type);
            *index += 1;
        }

        true
    }

    fn highlight_keywords(
        &mut self,
        index: &mut usize,
        chars: &[char],
        keywords: &[String],
        hl_type: highlighting::Type,
    ) -> bool {
        if *index > 0 {
            let prev_char = chars[*index - 1];
            if !is_separator(prev_char) {
                return false;
            }
        }

        for word in keywords {
            if *index < chars.len().saturating_sub(word.len()) {
                let next_char = chars[*index + word.len()];
                if !is_separator(next_char) {
                    continue;
                }
            }

            if self.highlight_substring(index, word, chars, hl_type) {
                return true;
            }
        }

        false
    }

    fn highlight_primary_keywords(
        &mut self,
        index: &mut usize,
        opts: &HighlightingOptions,
        chars: &[char],
    ) -> bool {
        self.highlight_keywords(
            index,
            chars,
            &opts.primary_keywords(),
            highlighting::Type::PrimaryKeywords,
        )
    }

    fn highlight_secondary_keywords(
        &mut self,
        index: &mut usize,
        opts: &HighlightingOptions,
        chars: &[char],
    ) -> bool {
        self.highlight_keywords(
            index,
            chars,
            &opts.secondary_keywords(),
            highlighting::Type::SecondaryKeywords,
        )
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.content.as_bytes()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

fn is_separator(c: char) -> bool {
    c.is_ascii_punctuation() || c.is_ascii_whitespace()
}
