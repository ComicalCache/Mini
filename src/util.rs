use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Error},
    path::{Path, PathBuf},
};

#[derive(PartialEq, Eq)]
pub enum CommandResult {
    Ok,
    ChangeBuffer(usize),
    SetAndChangeBuffer(usize, Vec<Cow<'static, str>>, Option<PathBuf>),
    Quit,
    ForceQuit,
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

pub fn read_file_to_lines(file: &mut File) -> Result<Vec<Cow<'static, str>>, Error> {
    BufReader::new(file)
        .lines()
        .map(|l| l.map(Cow::from))
        .collect::<Result<Vec<Cow<'static, str>>, _>>()
}
