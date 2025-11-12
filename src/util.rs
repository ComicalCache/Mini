use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io::Error,
    path::{Path, PathBuf},
};

#[derive(PartialEq, Eq)]
/// The result of a command entered by the user.
pub enum CommandResult {
    Ok,
    ChangeBuffer(usize),
    SetAndChangeBuffer(usize, String, Option<PathBuf>, Option<String>),
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

/// Retreives the filename of a given path.
pub fn file_name<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .file_name()
        .map(|p| p.to_string_lossy().to_string())
}

/// Opens a file as rw+truncate.
pub fn open_file<P: AsRef<Path>>(path: P) -> Result<File, Error> {
    // Create parent directories if they don't exist.
    let mut base = Path::new(path.as_ref());
    if !base.is_dir() {
        base = base.parent().unwrap_or_else(|| Path::new("/"));
    }
    std::fs::create_dir_all(base)?;

    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

/// Splits a string into a vector of lines.
pub fn split_to_lines<S: AsRef<str>>(data: S) -> Vec<Cow<'static, str>> {
    let mut buff = Vec::new();
    buff.extend(data.as_ref().lines().map(str::to_string).map(Cow::from));

    // lines() will discard a trailing empty line, but we don't want that.
    if data.as_ref().ends_with('\n') {
        buff.push(Cow::from(""));
    }

    buff
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
