use crate::util::{CursorMove, Mode, Position, ScreenDimensions};
use std::{
    fs::File,
    io::{BufRead, BufReader, Error, Seek, SeekFrom, Stdout, Write},
};
use termion::raw::{IntoRawMode, RawTerminal};

const PERCENTILE: usize = 7;
const TAB: &str = "    ";

pub struct State {
    screen_dims: ScreenDimensions,

    term_pos: Position,
    text_pos: Position,
    stdout: RawTerminal<Stdout>,

    pub mode: Mode,
    edited: bool,

    screen: Vec<String>,
    lines: Vec<String>,

    file: File,
}

impl State {
    pub fn new(width: usize, height: usize, mut file: File) -> Result<Self, Error> {
        let lines = BufReader::new(&mut file)
            .lines()
            .collect::<Result<Vec<String>, _>>()?;

        Ok(Self {
            screen_dims: ScreenDimensions {
                w: width,
                h: height,
            },
            term_pos: Position {
                x: 1,
                // Initialize cursor position at percentile of the screen height
                y: (height - 1) / PERCENTILE,
            },
            text_pos: Position { x: 0, y: 0 },
            stdout: std::io::stdout().into_raw_mode()?,
            mode: Mode::Command,
            edited: false,
            screen: vec![String::new(); height],
            lines: if lines.is_empty() {
                vec![String::new()]
            } else {
                lines
            },
            file,
        })
    }

    /// Changes to an alternate screen.
    pub fn init(&mut self) -> Result<(), Error> {
        use termion::{cursor::Goto, screen::ToAlternateScreen};

        // The values are set to valid defaults
        #[allow(clippy::cast_possible_truncation)]
        write!(
            self.stdout,
            "{ToAlternateScreen}{}",
            Goto(self.term_pos.x as u16, self.term_pos.y as u16)
        )?;
        self.stdout.flush()
    }

    /// Changes to the main screen.
    pub fn deinit(&mut self) -> Result<(), Error> {
        use termion::screen::ToMainScreen;

        write!(self.stdout, "{ToMainScreen}")?;
        self.stdout.flush()
    }

    /// Writes the file buffer to the file.
    pub fn write(&mut self) -> Result<(), Error> {
        if !self.edited {
            return Ok(());
        }
        self.edited = false;

        let size: usize = self.lines.iter().map(std::string::String::len).sum();
        let newlines = self.lines.len() - 1;
        self.file.set_len((size + newlines) as u64)?;

        self.file.seek(SeekFrom::Start(0))?;
        for line in &self.lines[..self.lines.len() - 1] {
            writeln!(self.file, "{line}")?;
        }
        write!(self.file, "{}", self.lines[self.lines.len() - 1])?;
        self.file.flush()
    }

    /// Handles a cursor move.
    pub fn move_cursor(&mut self, cursor_move: CursorMove, n: usize) {
        match cursor_move {
            CursorMove::Left => {
                self.term_pos.x = self.term_pos.x.saturating_sub(n).max(1);
                self.text_pos.x = self.text_pos.x.saturating_sub(n);
            }
            CursorMove::Down => {
                // Only move down if there is more text available
                let text_bound = self.lines.len();
                if self.text_pos.y + n < text_bound {
                    // Don't move down past the percentile of the screen height
                    self.term_pos.y = (self.term_pos.y + n)
                        .min((PERCENTILE - 1) * self.screen_dims.h / PERCENTILE);
                    self.text_pos.y += n;
                }
            }
            CursorMove::Up => {
                // Don't move up past percentile of the screen height
                self.term_pos.y = self
                    .term_pos
                    .y
                    .saturating_sub(n)
                    .max(self.screen_dims.h / PERCENTILE);
                self.text_pos.y = self.text_pos.y.saturating_sub(n);
            }
            CursorMove::Right => {
                // Only move right if there is more text available
                let line_bound = self.lines[self.text_pos.y].chars().count();
                if self.text_pos.x + n <= line_bound {
                    self.term_pos.x = (self.term_pos.x + n).min(self.screen_dims.w);
                    self.text_pos.x += n;
                }
            }
        }

        // When moving up and down, handle the case that one line contains less text than the current
        let line_bound = self.lines[self.text_pos.y].chars().count();
        if (cursor_move == CursorMove::Down || cursor_move == CursorMove::Up)
            && self.text_pos.x >= line_bound
        {
            let diff = self.text_pos.x - line_bound;
            self.text_pos.x = line_bound;
            self.term_pos.x = (self.term_pos.x.saturating_sub(diff)).max(1);
        }
    }

    fn set_text_line(&mut self, screen_idx: usize, lines_idx: usize) {
        let line = &self.lines[lines_idx];
        // Minus one since terminal coordinates start at 1
        let lower = self.text_pos.x.saturating_sub(self.term_pos.x - 1);
        // Only print lines if their content is visible on the screen (horizontal movement)
        if line.chars().count() > lower {
            let upper = (lower + self.screen_dims.w).min(line.chars().count());

            let start = line
                .char_indices()
                .nth(lower)
                .map(|(i, _)| i)
                // Safe to unwrap
                .unwrap();

            let end = line
                .char_indices()
                .nth(upper)
                // Use all remaining bytes if they don't fill the entire line
                .map_or(line.len(), |(i, _)| i);

            self.screen[screen_idx].replace_range(.., &line[start..end]);
        }
    }

    fn set_info_line(&mut self, screen_idx: usize) -> Result<(), std::fmt::Error> {
        use std::fmt::Write;

        let mode = match self.mode {
            Mode::Command => "C",
            Mode::Write => "W",
        };
        let edited = if self.edited { '*' } else { ' ' };
        let line = self.text_pos.y + 1;
        let col = self.text_pos.x + 1;
        let total = self.lines.len();
        let percentage = 100 * (self.text_pos.y + 1) / self.lines.len();
        let size: usize = self.lines.iter().map(std::string::String::len).sum();

        self.screen[screen_idx].clear();
        write!(
            &mut self.screen[screen_idx],
            "[{mode}] {line}:{col}/{total}[{percentage}%] [{size}B] {edited}",
        )
    }

    /// Prints the current state to the screen
    pub fn print_screen(&mut self) -> Result<(), Error> {
        use Mode::{Command, Write};
        use termion::{
            clear::All,
            cursor::{BlinkingBar, BlinkingBlock, Goto},
        };

        // Set info line
        if let Err(e) = self.set_info_line(0) {
            return Err(Error::new(std::io::ErrorKind::Other, e));
        }

        // Calculate which line of text is visible at what line on the screen
        #[allow(clippy::cast_possible_wrap)]
        let lines_offset = (self.text_pos.y + 1) as isize - self.term_pos.y as isize;
        // Plus one for info line offset
        for (screen_idx, lines_idx) in (1..self.screen_dims.h).zip(lines_offset + 1..) {
            self.screen[screen_idx].clear();

            // Skip screen lines outside the text line bounds
            // The value is guaranteed positive at that point
            #[allow(clippy::cast_sign_loss)]
            if lines_idx < 0 || (lines_idx as usize) >= self.lines.len() {
                continue;
            }

            // The value is guaranteed positive at that point
            #[allow(clippy::cast_sign_loss)]
            self.set_text_line(screen_idx, lines_idx as usize);
        }

        // Write the new content
        write!(self.stdout, "{All}{}", Goto(1, 1))?;
        for line in &self.screen[..self.screen.len() - 1] {
            write!(self.stdout, "{line}\n\r")?;
        }
        write!(self.stdout, "{}", self.screen[self.screen.len() - 1])?;

        // Write the new info line

        // Set the cursor to represent the current input mode
        // Since term_pos is always bounded by the screen dimensions it will never truncate
        #[allow(clippy::cast_possible_truncation)]
        match self.mode {
            Command => write!(
                self.stdout,
                "{}{BlinkingBlock}",
                Goto(self.term_pos.x as u16, self.term_pos.y as u16)
            )?,
            Write => write!(
                self.stdout,
                "{}{BlinkingBar}",
                Goto(self.term_pos.x as u16, self.term_pos.y as u16)
            )?,
        }

        self.stdout.flush()
    }

    /// Skip to the "next word" in the line
    pub fn next_word(&mut self) {
        let line = &self.lines[self.text_pos.y];
        // Return early if at end of line
        if line.chars().count() <= self.text_pos.x + 1 {
            return;
        }

        let Some(curr) = line.chars().nth(self.text_pos.x) else {
            unreachable!("Character must exist under cursor");
        };

        // Find next not alphanumeric character or alphanumeric character if the current character is not
        let Some((idx, c)) = line
            .chars()
            .skip(self.text_pos.x + 1)
            .enumerate()
            .find(|(_, c)| {
                !c.is_alphanumeric() || (!curr.is_alphanumeric() && c.is_alphanumeric())
            })
        else {
            // Return early if no "next word" candidate exists
            return;
        };

        if c.is_whitespace() {
            // Find next non-whitespace after whitespace
            let Some((jdx, _)) = line
                .chars()
                .skip(self.term_pos.x + 1 + idx)
                .enumerate()
                .find(|(_, c)| !c.is_whitespace())
            else {
                // Return early if after the whitespace there are no alphanumeric characters
                return;
            };

            // Move the cursor to the "next word"
            self.move_cursor(CursorMove::Right, idx + jdx + 2);
        } else {
            // If it is not whitespace set cursor to the position of the character
            self.move_cursor(CursorMove::Right, idx + 1);
        }
    }

    /// Skip to the "prev word" in the line
    pub fn prev_word(&mut self) {
        // Return early if already at start of line
        if self.text_pos.x == 0 {
            return;
        }

        let line = &self.lines[self.text_pos.y];

        // Find next non-whitespace character
        if let Some((idx, c)) = line
            .chars()
            .rev()
            .skip(line.chars().count() - self.text_pos.x)
            .enumerate()
            .find(|&(_, c)| !c.is_whitespace())
        {
            let mut offset = idx + 1;

            if c.is_alphanumeric() {
                // If it's alphanumeric, find first character of that sequence of alphanumeric characters
                offset += line
                    .chars()
                    .rev()
                    .skip(line.chars().count() - self.text_pos.x)
                    .skip(idx + 1)
                    .take_while(|&c| c.is_alphanumeric())
                    .count();
            }

            self.move_cursor(CursorMove::Left, offset);
        } else {
            // Move to the start of line
            self.move_cursor(CursorMove::Left, self.text_pos.x);
        }
    }

    /// Jumps to the start of a line
    pub fn jump_to_start_of_line(&mut self) {
        self.move_cursor(
            CursorMove::Left,
            self.lines[self.text_pos.y].chars().count(),
        );
    }

    /// Jumps to the last character of a line
    pub fn jump_to_end_of_line(&mut self) {
        self.move_cursor(
            CursorMove::Right,
            self.lines[self.text_pos.y]
                .chars()
                .count()
                .saturating_sub(self.text_pos.x + 1),
        );
    }

    /// Inserts a new line above the current line and moves to it
    pub fn insert_move_new_line_above(&mut self) {
        self.lines.insert(self.text_pos.y, String::new());
        // No need to move since the cursor pos stays the same
        self.edited = true;
    }

    /// Inserts a new line bellow the current line and moves to it
    pub fn insert_move_new_line_bellow(&mut self) {
        self.lines.insert(self.text_pos.y + 1, String::new());
        self.move_cursor(CursorMove::Down, 1);
        self.edited = true;
    }

    /// Writes a character to the buffer
    pub fn write_char(&mut self, c: char) {
        let idx = self.lines[self.text_pos.y]
            .char_indices()
            .nth(self.text_pos.x)
            .map_or(self.lines[self.text_pos.y].len(), |(i, _)| i);
        self.lines[self.text_pos.y].insert(idx, c);

        self.text_pos.x += 1;
        self.term_pos.x = (self.term_pos.x + 1).min(self.screen_dims.w);
        self.edited = true;
    }

    /// Writes a new line to the buffer, splitting an existing line if necessary
    pub fn write_new_line(&mut self) {
        let line = &mut self.lines[self.text_pos.y];
        let idx = line
            .char_indices()
            .nth(self.text_pos.x)
            .map_or(line.len(), |(i, _)| i);

        let new_line = line.split_off(idx);
        self.lines.insert(self.text_pos.y + 1, new_line);

        self.move_cursor(CursorMove::Down, 1);
        self.move_cursor(CursorMove::Left, self.text_pos.x);
        self.edited = true;
    }

    /// Writes a tab character to the buffer
    pub fn write_tab(&mut self) {
        let idx = self.lines[self.text_pos.y]
            .char_indices()
            .nth(self.text_pos.x)
            .map_or(self.lines[self.text_pos.y].len(), |(i, _)| i);
        self.lines[self.text_pos.y].insert_str(idx, TAB);

        self.move_cursor(CursorMove::Right, TAB.chars().count());
        self.edited = true;
    }

    /// Deletes a character from the buffer, joining two lines if necessary
    pub fn delete_char(&mut self) {
        if self.text_pos.x > 0 {
            // If deleting a character in a line
            let line = &mut self.lines[self.text_pos.y];
            let idx = line
                .char_indices()
                .nth(self.text_pos.x - 1)
                .map(|(i, _)| i)
                // Safe to unwrap
                .unwrap();

            line.remove(idx);
            self.move_cursor(CursorMove::Left, 1);
            self.edited = true;
        } else if self.text_pos.y > 0 {
            // If deleting at the beginning of a line (don't delete the first line)
            let prev_line_len = self.lines[self.text_pos.y - 1].chars().count();
            let line = self.lines.remove(self.text_pos.y);
            self.lines[self.text_pos.y - 1].push_str(&line);

            self.move_cursor(CursorMove::Up, 1);
            self.move_cursor(CursorMove::Right, prev_line_len);
            self.edited = true;
        }
    }

    fn find_matching_bracket(&self) -> Option<Position> {
        let Some(current_char) = self.lines[self.text_pos.y].chars().nth(self.text_pos.x) else {
            return None; // Cursor is at the end of line
        };

        let (opening, closing, forward) = match current_char {
            '(' => ('(', ')', true),
            '[' => ('[', ']', true),
            '{' => ('{', '}', true),
            '<' => ('<', '>', true),
            ')' => ('(', ')', false),
            ']' => ('[', ']', false),
            '}' => ('{', '}', false),
            '>' => ('<', '>', false),
            _ => return None,
        };

        let mut depth = 1;
        if forward {
            // Search forward from the character after the cursor
            for y in self.text_pos.y..self.lines.len() {
                let line = &self.lines[y];
                let offset = if y == self.text_pos.y {
                    self.text_pos.x + 1
                } else {
                    0
                };

                for (x, c) in line.char_indices().skip(offset) {
                    if c == opening {
                        depth += 1;
                    } else if c == closing {
                        depth -= 1;
                    }

                    if depth == 0 {
                        return Some(Position { x, y });
                    }
                }
            }
        } else {
            // Search backward from the character before the cursor
            for y in (0..=self.text_pos.y).rev() {
                let line = &self.lines[y];
                let offset = if y == self.text_pos.y {
                    line.chars().count() - self.text_pos.x
                } else {
                    0
                };

                for (x, c) in line.char_indices().rev().skip(offset) {
                    if c == closing {
                        depth += 1;
                    } else if c == opening {
                        depth -= 1;
                    }

                    if depth == 0 {
                        return Some(Position { x, y });
                    }
                }
            }
        }

        None
    }

    /// Jumps to the matching opposite bracket if on a bracket
    pub fn jump_to_matching_opposite(&mut self) {
        let Some(Position { x, y }) = self.find_matching_bracket() else {
            return;
        };

        if y < self.text_pos.y {
            self.move_cursor(CursorMove::Up, self.text_pos.y - y);
        } else if y > self.text_pos.y {
            self.move_cursor(CursorMove::Down, y - self.text_pos.y);
        }

        if x < self.text_pos.x {
            self.move_cursor(CursorMove::Left, self.text_pos.x - x);
        } else if x > self.text_pos.x {
            self.move_cursor(CursorMove::Right, x - self.text_pos.x);
        }
    }
}
