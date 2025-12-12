use std::{
    fs::{File, OpenOptions},
    io::Error,
    path::Path,
};

pub const TAB_WIDTH: usize = 4;

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
