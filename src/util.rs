use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Error},
    path::Path,
};

#[derive(PartialEq, Eq)]
pub enum CommandResult {
    Ok,
    ChangeBuffer(usize),
    SetAndChangeBuffer(usize, Vec<String>),
    Quit,
}

#[derive(Clone, Copy)]
pub enum CursorStyle {
    BlinkingBar,
    BlinkingBlock,
    SteadyBlock,
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<File, Error> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

pub fn read_file_to_lines(file: &mut File) -> Result<Vec<String>, Error> {
    BufReader::new(file)
        .lines()
        .collect::<Result<Vec<String>, _>>()
}
