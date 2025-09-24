mod apply_cmd;
mod cursor;
mod edit;
mod r#move;
mod print_screen;

use crate::util::{Mode, Position, ScreenDimensions};
use std::fs::File;

pub struct Buffer {
    buff_name: &'static str,
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
    pub fn new(
        buff_name: &'static str,
        width: usize,
        height: usize,
        mut line_buff: Vec<String>,
        file: Option<File>,
    ) -> Self {
        if line_buff.is_empty() {
            line_buff.push(String::new());
        }

        Self {
            buff_name,
            screen_dims: ScreenDimensions {
                w: width,
                h: height,
            },
            // Initialize cursor position at middle of screen where it is fixed
            term_content_pos: Position::new((height - 1) / 2, 1),
            term_cmd_pos: Position::new(height, 1),
            mode: Mode::View,
            edited: false,
            select: None,
            screen_buff: vec![String::new(); height],
            txt_pos: Position::new(0, 0),
            line_buff,
            cmd_pos: Position::new(0, 0),
            cmd_buff: String::new(),
            file,
        }
    }

    // Reinit the buffer
    pub fn reinit(&mut self) {
        // Initialize cursor position at middle of screen where it is fixed
        self.term_content_pos = Position::new((self.screen_dims.h - 1) / 2, 1);
        self.term_cmd_pos = Position::new(self.screen_dims.h, 1);
        self.mode = Mode::View;
        self.edited = false;
        self.txt_pos = Position::new(0, 0);
        self.line_buff.truncate(1);
        self.line_buff[0].clear();
        self.cmd_pos = Position::new(0, 0);
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
}
