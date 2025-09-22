use crate::util::{CmdResult, CursorMove, Mode, Position, ScreenDimensions};
use std::{
    fs::File,
    io::{BufRead, BufReader, Error, Seek, SeekFrom, Stdout, Write},
};
use termion::raw::{IntoRawMode, RawTerminal};

const PERCENTILE: usize = 7;
const TAB: &str = "    ";

pub struct State {
    screen_dims: ScreenDimensions,

    term_content_pos: Position,
    term_cmd_pos: Position,

    mode: Mode,
    edited: bool,

    screen_buff: Vec<String>,

    txt_pos: Position,
    line_buff: Vec<String>,

    cmd_pos: Position,
    cmd_buff: String,

    stdout: RawTerminal<Stdout>,
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
            term_content_pos: Position {
                x: 1,
                // Initialize cursor position at percentile of the screen height
                y: (height - 1) / PERCENTILE,
            },
            term_cmd_pos: Position { x: 1, y: height },
            mode: Mode::View,
            edited: false,
            screen_buff: vec![String::new(); height],
            txt_pos: Position { x: 0, y: 0 },
            line_buff: if lines.is_empty() {
                vec![String::new()]
            } else {
                lines
            },
            cmd_pos: Position { x: 0, y: 0 },
            cmd_buff: String::new(),
            stdout: std::io::stdout().into_raw_mode()?,
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
            Goto(
                self.term_content_pos.x as u16,
                self.term_content_pos.y as u16
            )
        )?;
        self.stdout.flush()
    }

    /// Changes to the main screen.
    pub fn deinit(&mut self) -> Result<(), Error> {
        use termion::screen::ToMainScreen;

        write!(self.stdout, "{ToMainScreen}")?;
        self.stdout.flush()
    }

    /// Applies the command entered during command mode
    pub fn apply_cmd(&mut self) -> Result<CmdResult, Error> {
        use CmdResult::{Continue, Quit};

        match self.cmd_buff.as_str() {
            "q" => {
                if !self.edited {
                    return Ok(Quit);
                }

                // TODO: fix this. There should be a buffer for errors that is somehow shown...
                self.cmd_buff.replace_range(
                    ..,
                    "There are unsafed changes, save or qq to force quit (esc to close this error)",
                );
                self.cmd_pos.x = self.cmd_buff.chars().count();
                self.term_cmd_pos.x = self.cmd_buff.chars().count() + 1;
            }
            "qq" => return Ok(Quit),
            "wq" => {
                self.write()?;
                return Ok(Quit);
            }
            "w" => {
                self.write()?;
                self.change_mode(Mode::View);
            }
            _ => {
                // Unrecognized command
                // TODO: this should display an error, see above TODO for error buffer
                self.change_mode(Mode::View);
            }
        }

        Ok(Continue)
    }

    /// Get the current mode
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Set the mode
    pub fn change_mode(&mut self, mode: Mode) {
        use Mode::{Command, View, Write};

        match self.mode {
            Command => {
                self.cmd_buff.clear();
                self.cmd_pos.x = 0;
                self.term_cmd_pos.x = 1;
            }
            View | Write => {}
        }

        self.mode = mode;
    }

    /// Writes the file buffer to the file.
    pub fn write(&mut self) -> Result<(), Error> {
        if !self.edited {
            return Ok(());
        }
        self.edited = false;

        let size: usize = self.line_buff.iter().map(std::string::String::len).sum();
        let newlines = self.line_buff.len() - 1;
        self.file.set_len((size + newlines) as u64)?;

        self.file.seek(SeekFrom::Start(0))?;
        for line in &self.line_buff[..self.line_buff.len() - 1] {
            writeln!(self.file, "{line}")?;
        }
        write!(self.file, "{}", self.line_buff[self.line_buff.len() - 1])?;
        self.file.flush()
    }

    /// Handles a cursor move.
    pub fn move_cursor(&mut self, cursor_move: CursorMove, n: usize) {
        match cursor_move {
            CursorMove::Left => {
                self.term_content_pos.x = self.term_content_pos.x.saturating_sub(n).max(1);
                self.txt_pos.x = self.txt_pos.x.saturating_sub(n);
            }
            CursorMove::Down => {
                // Only move down if there is more text available
                let text_bound = self.line_buff.len();
                if self.txt_pos.y + n < text_bound {
                    // Don't move down past the percentile of the screen height
                    self.term_content_pos.y = (self.term_content_pos.y + n)
                        .min((PERCENTILE - 1) * self.screen_dims.h / PERCENTILE);
                    self.txt_pos.y = (self.txt_pos.y + n).min(text_bound.saturating_sub(1));
                }
            }
            CursorMove::Up => {
                // Don't move up past percentile of the screen height
                self.term_content_pos.y = self
                    .term_content_pos
                    .y
                    .saturating_sub(n)
                    .max(self.screen_dims.h / PERCENTILE);
                self.txt_pos.y = self.txt_pos.y.saturating_sub(n);
            }
            CursorMove::Right => {
                // Only move right if there is more text available
                let line_bound = self.line_buff[self.txt_pos.y].chars().count();
                if self.txt_pos.x + n <= line_bound {
                    self.term_content_pos.x = (self.term_content_pos.x + n).min(self.screen_dims.w);
                    self.txt_pos.x = (self.txt_pos.x + n).min(line_bound);
                }
            }
        }

        // When moving up and down, handle the case that one line contains less text than the current
        let line_bound = self.line_buff[self.txt_pos.y].chars().count();
        if (cursor_move == CursorMove::Down || cursor_move == CursorMove::Up)
            && self.txt_pos.x >= line_bound
        {
            let diff = self.txt_pos.x - line_bound;
            self.txt_pos.x = line_bound;
            self.term_content_pos.x = (self.term_content_pos.x.saturating_sub(diff)).max(1);
        }
    }

    /// Moves the cursor when in command mode
    pub fn move_cmd_cursor(&mut self, cursor_move: CursorMove, n: usize) {
        self.term_cmd_pos.y = self.screen_dims.h;

        match cursor_move {
            CursorMove::Left => {
                self.term_cmd_pos.x = self.term_cmd_pos.x.saturating_sub(n).max(1);
                self.cmd_pos.x = self.cmd_pos.x.saturating_sub(n);
            }
            CursorMove::Right => {
                self.term_cmd_pos.x = (self.term_cmd_pos.x + n).min(self.screen_dims.w);
                self.cmd_pos.x = (self.cmd_pos.x + n).min(self.cmd_buff.chars().count());
            }
            _ => {
                // TODO: add command history to scroll
            }
        }
    }

    fn set_text_line(&mut self, screen_idx: usize, lines_idx: usize) {
        let line = &self.line_buff[lines_idx];
        // Minus one since terminal coordinates start at 1
        let lower = self.txt_pos.x.saturating_sub(self.term_content_pos.x - 1);
        // Only print lines if their content is visible on the screen (horizontal movement)
        if line.chars().count() > lower {
            let upper = (lower + self.screen_dims.w).min(line.chars().count());

            let start = line
                .char_indices()
                .nth(lower)
                .map(|(idx, _)| idx)
                // Safe to unwrap
                .unwrap();

            let end = line
                .char_indices()
                .nth(upper)
                // Use all remaining bytes if they don't fill the entire line
                .map_or(line.len(), |(idx, _)| idx);

            self.screen_buff[screen_idx].replace_range(.., &line[start..end]);
        }
    }

    fn set_info_line(&mut self, screen_idx: usize) -> Result<(), std::fmt::Error> {
        use std::fmt::Write;

        let mode = match self.mode {
            Mode::View => "V",
            Mode::Write => "W",
            Mode::Command => "C",
        };
        let edited = if self.edited { '*' } else { ' ' };
        let line = self.txt_pos.y + 1;
        let col = self.txt_pos.x + 1;
        let total = self.line_buff.len();
        let percentage = 100 * (self.txt_pos.y + 1) / self.line_buff.len();
        let size: usize = self.line_buff.iter().map(std::string::String::len).sum();

        self.screen_buff[screen_idx].clear();
        write!(
            &mut self.screen_buff[screen_idx],
            "[{mode}] {line}:{col}/{total}[{percentage}%] [{size}B] {edited}",
        )
    }

    fn set_cmd_line(&mut self, screen_idx: usize) {
        self.screen_buff[screen_idx].clear();
        self.screen_buff[screen_idx].clone_from(&self.cmd_buff);
    }

    /// Prints the current state to the screen
    pub fn print_screen(&mut self) -> Result<(), Error> {
        use Mode::{Command, View, Write};
        use termion::{
            clear::All,
            cursor::{BlinkingBar, BlinkingBlock, Goto},
        };

        // Set info line
        if let Err(err) = self.set_info_line(0) {
            return Err(Error::other(err));
        }

        // Calculate which line of text is visible at what line on the screen
        #[allow(clippy::cast_possible_wrap)]
        let lines_offset = (self.txt_pos.y + 1) as isize - self.term_content_pos.y as isize;

        // Plus one for info line offset
        for (screen_idx, lines_idx) in (1..self.screen_dims.h).zip(lines_offset + 1..) {
            self.screen_buff[screen_idx].clear();

            // Skip screen lines outside the text line bounds
            // The value is guaranteed positive at that point
            #[allow(clippy::cast_sign_loss)]
            if lines_idx < 0 || (lines_idx as usize) >= self.line_buff.len() {
                continue;
            }

            // The value is guaranteed positive at that point
            #[allow(clippy::cast_sign_loss)]
            self.set_text_line(screen_idx, lines_idx as usize);
        }

        // Set command line if in command mode
        if self.mode == Command {
            self.set_cmd_line(self.screen_dims.h - 1);
        }

        // Write the new content
        write!(self.stdout, "{All}{}", Goto(1, 1))?;
        for line in &self.screen_buff[..self.screen_buff.len() - 1] {
            write!(self.stdout, "{line}\n\r")?;
        }
        write!(
            self.stdout,
            "{}",
            self.screen_buff[self.screen_buff.len() - 1]
        )?;

        // Write the new info line

        // Set the cursor to represent the current input mode
        // Since term_pos is always bounded by the screen dimensions it will never truncate
        #[allow(clippy::cast_possible_truncation)]
        match self.mode {
            View => write!(
                self.stdout,
                "{}{BlinkingBlock}",
                Goto(
                    self.term_content_pos.x as u16,
                    self.term_content_pos.y as u16
                )
            )?,
            Write => write!(
                self.stdout,
                "{}{BlinkingBar}",
                Goto(
                    self.term_content_pos.x as u16,
                    self.term_content_pos.y as u16
                )
            )?,
            Command => write!(
                self.stdout,
                "{}{BlinkingBar}",
                Goto(self.term_cmd_pos.x as u16, self.term_cmd_pos.y as u16)
            )?,
        }

        self.stdout.flush()
    }

    /// Skip to the "next word" in the line
    pub fn next_word(&mut self) {
        let line = &self.line_buff[self.txt_pos.y];
        // Return early if at end of line
        if line.chars().count() <= self.txt_pos.x + 1 {
            return;
        }

        let Some(curr) = line.chars().nth(self.txt_pos.x) else {
            unreachable!("Character must exist under cursor");
        };

        // Find next not alphanumeric character or alphanumeric character if the current character is not
        let Some((idx, ch)) = line
            .chars()
            .skip(self.txt_pos.x + 1)
            .enumerate()
            .find(|(_, ch)| {
                !ch.is_alphanumeric() || (!curr.is_alphanumeric() && ch.is_alphanumeric())
            })
        else {
            // Return early if no "next word" candidate exists
            return;
        };

        if ch.is_whitespace() {
            // Find next non-whitespace after whitespace
            let Some((jdx, _)) = line
                .chars()
                .skip(self.term_content_pos.x + 1 + idx)
                .enumerate()
                .find(|(_, ch)| !ch.is_whitespace())
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
        if self.txt_pos.x == 0 {
            return;
        }

        let line = &self.line_buff[self.txt_pos.y];

        // Find next non-whitespace character
        if let Some((idx, ch)) = line
            .chars()
            .rev()
            .skip(line.chars().count() - self.txt_pos.x)
            .enumerate()
            .find(|&(_, ch)| !ch.is_whitespace())
        {
            let mut offset = idx + 1;

            if ch.is_alphanumeric() {
                // If it's alphanumeric, find first character of that sequence of alphanumeric characters
                offset += line
                    .chars()
                    .rev()
                    .skip(line.chars().count() - self.txt_pos.x)
                    .skip(idx + 1)
                    .take_while(|&ch| ch.is_alphanumeric())
                    .count();
            }

            self.move_cursor(CursorMove::Left, offset);
        } else {
            // Move to the start of line
            self.move_cursor(CursorMove::Left, self.txt_pos.x);
        }
    }

    /// Jumps to the start of a line
    pub fn jump_to_start_of_line(&mut self) {
        self.move_cursor(
            CursorMove::Left,
            self.line_buff[self.txt_pos.y].chars().count(),
        );
    }

    /// Jumps to the last character of a line
    pub fn jump_to_end_of_line(&mut self) {
        self.move_cursor(
            CursorMove::Right,
            self.line_buff[self.txt_pos.y]
                .chars()
                .count()
                .saturating_sub(self.txt_pos.x + 1),
        );
    }

    /// Inserts a new line above the current line and moves to it
    pub fn insert_move_new_line_above(&mut self) {
        self.line_buff.insert(self.txt_pos.y, String::new());
        // No need to move since the cursor pos stays the same
        self.edited = true;
    }

    /// Inserts a new line bellow the current line and moves to it
    pub fn insert_move_new_line_bellow(&mut self) {
        self.line_buff.insert(self.txt_pos.y + 1, String::new());
        self.move_cursor(CursorMove::Down, 1);
        self.edited = true;
    }

    /// Writes a character to the buffer
    pub fn write_char(&mut self, ch: char) {
        let idx = self.line_buff[self.txt_pos.y]
            .char_indices()
            .nth(self.txt_pos.x)
            .map_or(self.line_buff[self.txt_pos.y].len(), |(idx, _)| idx);
        self.line_buff[self.txt_pos.y].insert(idx, ch);

        self.txt_pos.x += 1;
        self.term_content_pos.x = (self.term_content_pos.x + 1).min(self.screen_dims.w);
        self.edited = true;
    }

    /// Writes a character to the command buffer
    pub fn write_cmd_char(&mut self, ch: char) {
        let idx = self
            .cmd_buff
            .char_indices()
            .nth(self.cmd_pos.x)
            .map_or(self.cmd_buff.len(), |(idx, _)| idx);
        self.cmd_buff.insert(idx, ch);

        self.cmd_pos.x += 1;
        self.term_cmd_pos.x = (self.cmd_pos.x + 1).min(self.screen_dims.w);
    }

    /// Writes a new line to the buffer, splitting an existing line if necessary
    pub fn write_new_line(&mut self) {
        let line = &mut self.line_buff[self.txt_pos.y];
        let idx = line
            .char_indices()
            .nth(self.txt_pos.x)
            .map_or(line.len(), |(idx, _)| idx);

        let new_line = line.split_off(idx);
        self.line_buff.insert(self.txt_pos.y + 1, new_line);

        self.move_cursor(CursorMove::Down, 1);
        self.move_cursor(CursorMove::Left, self.txt_pos.x);
        self.edited = true;
    }

    /// Writes a tab character to the buffer
    pub fn write_tab(&mut self) {
        let idx = self.line_buff[self.txt_pos.y]
            .char_indices()
            .nth(self.txt_pos.x)
            .map_or(self.line_buff[self.txt_pos.y].len(), |(idx, _)| idx);
        self.line_buff[self.txt_pos.y].insert_str(idx, TAB);

        self.move_cursor(CursorMove::Right, TAB.chars().count());
        self.edited = true;
    }

    /// Writes a tab character to the command buffer
    pub fn write_cmd_tab(&mut self) {
        let idx = self
            .cmd_buff
            .char_indices()
            .nth(self.cmd_pos.x)
            .map_or(self.cmd_buff.len(), |(idx, _)| idx);
        self.cmd_buff.insert_str(idx, TAB);

        self.move_cmd_cursor(CursorMove::Right, TAB.chars().count());
    }

    /// Deletes a character from the buffer, joining two lines if necessary
    pub fn delete_char(&mut self) {
        if self.txt_pos.x > 0 {
            // If deleting a character in a line
            let line = &mut self.line_buff[self.txt_pos.y];
            let idx = line
                .char_indices()
                .nth(self.txt_pos.x - 1)
                .map(|(idx, _)| idx)
                // Safe to unwrap
                .unwrap();

            line.remove(idx);
            self.move_cursor(CursorMove::Left, 1);
            self.edited = true;
        } else if self.txt_pos.y > 0 {
            // If deleting at the beginning of a line (don't delete the first line)
            let prev_line_len = self.line_buff[self.txt_pos.y - 1].chars().count();
            let line = self.line_buff.remove(self.txt_pos.y);
            self.line_buff[self.txt_pos.y - 1].push_str(&line);

            self.move_cursor(CursorMove::Up, 1);
            self.move_cursor(CursorMove::Right, prev_line_len);
            self.edited = true;
        }
    }

    /// Deletes a character from the command buffer
    pub fn delete_cmd_char(&mut self) {
        if self.cmd_pos.x > 0 {
            let idx = self
                .cmd_buff
                .char_indices()
                .nth(self.cmd_pos.x - 1)
                .map(|(idx, _)| idx)
                // Safe to unwrap
                .unwrap();

            self.cmd_buff.remove(idx);
            self.move_cmd_cursor(CursorMove::Left, 1);
        }
    }

    fn find_matching_bracket(&self) -> Option<Position> {
        let Some(current_char) = self.line_buff[self.txt_pos.y].chars().nth(self.txt_pos.x) else {
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
            for y in self.txt_pos.y..self.line_buff.len() {
                let line = &self.line_buff[y];
                let offset = if y == self.txt_pos.y {
                    self.txt_pos.x + 1
                } else {
                    0
                };

                for (x, ch) in line.char_indices().skip(offset) {
                    if ch == opening {
                        depth += 1;
                    } else if ch == closing {
                        depth -= 1;
                    }

                    if depth == 0 {
                        return Some(Position { x, y });
                    }
                }
            }
        } else {
            // Search backward from the character before the cursor
            for y in (0..=self.txt_pos.y).rev() {
                let line = &self.line_buff[y];
                let offset = if y == self.txt_pos.y {
                    line.chars().count() - self.txt_pos.x
                } else {
                    0
                };

                for (x, ch) in line.char_indices().rev().skip(offset) {
                    if ch == closing {
                        depth += 1;
                    } else if ch == opening {
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

        if y < self.txt_pos.y {
            self.move_cursor(CursorMove::Up, self.txt_pos.y - y);
        } else if y > self.txt_pos.y {
            self.move_cursor(CursorMove::Down, y - self.txt_pos.y);
        }

        if x < self.txt_pos.x {
            self.move_cursor(CursorMove::Left, self.txt_pos.x - x);
        } else if x > self.txt_pos.x {
            self.move_cursor(CursorMove::Right, x - self.txt_pos.x);
        }
    }
}
