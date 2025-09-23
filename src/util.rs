use std::{
    fs::File,
    io::{BufRead, BufReader, Error},
};

pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    View,
    Write,
    Command,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CursorMove {
    Left,
    Down,
    Up,
    Right,
}

#[derive(Clone, PartialEq, Eq)]
pub enum CmdResult {
    Quit,
    Continue,
    Error(String),
}

pub struct ScreenDimensions {
    pub w: usize,
    pub h: usize,
}

/// Reads a file to a vec of strings
pub fn read_file(file: &mut File) -> Result<Vec<String>, Error> {
    BufReader::new(file)
        .lines()
        .collect::<Result<Vec<String>, _>>()
}
