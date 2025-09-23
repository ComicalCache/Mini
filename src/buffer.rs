mod apply_cmd;
mod cursor;
mod edit;
mod r#move;
mod print_screen;

use crate::util::{Mode, Position, ScreenDimensions};
use std::{
    fs::File,
    io::{Error, Seek, SeekFrom, Write},
};

pub struct Buffer {
    screen_dims: ScreenDimensions,

    term_content_pos: Position,
    term_cmd_pos: Position,

    mode: Mode,
    edited: bool,
    select: Option<Position>,

    screen_buff: Vec<String>,

    txt_pos: Position,
    line_buff: Vec<String>,

    cmd_pos: Position,
    cmd_buff: String,

    file: Option<File>,
}

impl Buffer {
    pub fn new(width: usize, height: usize, line_buff: Vec<String>, file: Option<File>) -> Self {
        Self {
            screen_dims: ScreenDimensions {
                w: width,
                h: height,
            },
            term_content_pos: Position {
                x: 1,
                // Initialize cursor position at middle of screen where it is fixed
                y: (height - 1) / 2,
            },
            term_cmd_pos: Position { x: 1, y: height },
            mode: Mode::View,
            edited: false,
            select: None,
            screen_buff: vec![String::new(); height],
            txt_pos: Position { x: 0, y: 0 },
            line_buff,
            cmd_pos: Position { x: 0, y: 0 },
            cmd_buff: String::new(),
            file,
        }
    }

    pub fn reinit(&mut self) {
        self.term_content_pos = Position {
            x: 1,
            // Initialize cursor position at middle of screen where it is fixed
            y: (self.screen_dims.h - 1) / 2,
        };
        self.term_cmd_pos = Position {
            x: 1,
            y: self.screen_dims.h,
        };
        self.mode = Mode::View;
        self.edited = false;
        self.txt_pos = Position { x: 0, y: 0 };
        self.cmd_pos = Position { x: 0, y: 0 };
        self.cmd_buff.clear();
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

    /// Sets the internal line buffer with new contents
    pub fn set_line_buff(&mut self, contents: &str) {
        self.line_buff = contents
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>();
    }

    /// Marks the position at which selection began
    pub fn set_select(&mut self) {
        self.select = Some(self.txt_pos);
    }

    /// Resets the select position
    pub fn reset_select(&mut self) {
        self.select = None;
    }

    /// Writes the file buffer to the file.
    pub fn write_to_file(&mut self) -> Result<bool, Error> {
        if !self.edited {
            return Ok(true);
        }
        self.edited = false;

        let Some(file) = self.file.as_mut() else {
            return Ok(false);
        };

        let size: usize = self.line_buff.iter().map(String::len).sum();
        let newlines = self.line_buff.len() - 1;
        file.set_len((size + newlines) as u64)?;

        file.seek(SeekFrom::Start(0))?;
        for line in &self.line_buff[..self.line_buff.len() - 1] {
            writeln!(file, "{line}")?;
        }
        write!(file, "{}", self.line_buff[self.line_buff.len() - 1])?;
        file.flush()?;

        Ok(true)
    }
}
