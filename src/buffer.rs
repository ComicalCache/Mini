mod apply_cmd;
mod cursor;
mod edit;
mod r#move;
mod print_screen;

use crate::util::{CmdResult, Mode, Position, ScreenDimensions};
use std::{
    fs::File,
    io::{BufWriter, Seek, SeekFrom, Write},
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

    // Reinit the buffer
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
        self.line_buff.truncate(1);
        self.line_buff[0].clear();
        self.cmd_pos = Position { x: 0, y: 0 };
        self.cmd_buff.clear();
        self.file = None;
    }

    /// Updates the buffer on terminal resize
    pub fn update_screen_dimentions(&mut self, width: usize, height: usize) {
        if self.screen_dims.w == width && self.screen_dims.h == height {
            return;
        }

        self.screen_dims.w = width;
        self.screen_dims.h = height;

        self.term_content_pos.y = (height - 1) / 2;
        self.term_cmd_pos.y = height;

        self.term_content_pos.x = self.term_content_pos.x.min(width);
        self.term_cmd_pos.x = self.term_cmd_pos.x.min(width);

        self.screen_buff.resize(height, String::new());
    }

    /// Get the current mode
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Set the mode
    pub fn change_mode(&mut self, mode: Mode) {
        match self.mode {
            Mode::Command => {
                self.cmd_buff.clear();
                self.cmd_pos.x = 0;
                self.term_cmd_pos.x = 1;
            }
            Mode::View | Mode::Write => {}
        }

        self.mode = mode;
    }

    /// Sets the internal line buffer with new contents
    pub fn set_line_buff(&mut self, contents: &str) {
        let lines = contents.lines().count();
        if lines == 0 {
            // Line buffer always has at least one entry
            self.line_buff.truncate(1);
            self.line_buff[0].clear();
            return;
        }

        self.line_buff.resize(lines, String::new());
        for (idx, line) in contents.lines().enumerate() {
            self.line_buff[idx].replace_range(.., line);
        }
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
    pub fn write_to_file(&mut self) -> Result<bool, CmdResult> {
        if !self.edited {
            return Ok(true);
        }

        let Some(file) = self.file.as_mut() else {
            return Ok(false);
        };

        let size: u64 = self.line_buff.iter().map(|s| s.len() as u64 + 1).sum();
        if let Err(err) = file.set_len(size.saturating_sub(1)) {
            return Err(CmdResult::Info(err.to_string()));
        }

        if let Err(err) = file.seek(SeekFrom::Start(0)) {
            return Err(CmdResult::Info(err.to_string()));
        }
        let mut writer = BufWriter::new(file);
        for line in &self.line_buff {
            if let Err(err) = writeln!(writer, "{line}") {
                return Err(CmdResult::Info(err.to_string()));
            }
        }
        if let Err(err) = writer.flush() {
            return Err(CmdResult::Info(err.to_string()));
        }

        self.edited = false;
        Ok(true)
    }
}
