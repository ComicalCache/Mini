use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Error},
    path::{Path, PathBuf},
};

#[derive(PartialEq, Eq)]
/// The result of a command entered by the user.
pub enum CommandResult {
    Ok,
    ChangeBuffer(usize),
    SetAndChangeBuffer(usize, Vec<Cow<'static, str>>, Option<PathBuf>),
    Quit,
    ForceQuit,
}

#[derive(Clone, Copy)]
/// The displayed cursor style.
pub enum CursorStyle {
    BlinkingBar,
    BlinkingBlock,
    SteadyBlock,
}

/// Opens a file as rw+truncate.
pub fn open_file<P: AsRef<Path>>(path: P) -> Result<File, Error> {
    // Create parent directories if they don't exist.
    let mut base = Path::new(path.as_ref());
    if !base.is_dir() {
        base = base.parent().unwrap_or(Path::new("/"));
    }
    std::fs::create_dir_all(base)?;

    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

/// Reads a files contents into lines.
pub fn read_file_to_lines(file: &mut File) -> Result<Vec<Cow<'static, str>>, Error> {
    BufReader::new(file)
        .lines()
        .map(|l| l.map(Cow::from))
        .collect::<Result<Vec<Cow<'static, str>>, _>>()
}

/// Parses a line column string 'y:x' where y is the line and x is the column.
pub fn line_column(input: &str) -> (Option<usize>, Option<usize>) {
    let mut y: Option<usize> = None;
    let mut x: Option<usize> = None;

    if let Some((y_str, x_str)) = input.split_once(':') {
        y = y_str.parse::<usize>().ok();
        x = x_str.parse::<usize>().ok();
    } else if !input.is_empty() {
        // Only y was supplied.
        y = input.parse::<usize>().ok();
    }

    (x, y)
}
