use std::{
    fs::{File, OpenOptions},
    io::Error,
    path::Path,
};

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

use termion::event::Key;

/// Converts a `Key` to a String.
pub fn key_to_string(key: Key) -> Option<String> {
    match key {
        Key::Char(c) => Some(c.to_string()),

        // Map 'ctrl+a' (97) to 1, 'ctrl+b' to 2, etc.
        Key::Ctrl(c) => {
            let byte = c as u8;
            Some(((byte & 0x1f) as char).to_string())
        }

        // Map 'alt+_' to control-sequence with char.
        Key::Alt(c) => Some(format!("\x1b{c}")),

        // Common special keys mapped to standard ANSI escape sequences
        Key::Backspace => Some("\x7f".to_string()),
        Key::Left => Some("\x1b[D".to_string()),
        Key::Right => Some("\x1b[C".to_string()),
        Key::Up => Some("\x1b[A".to_string()),
        Key::Down => Some("\x1b[B".to_string()),
        Key::Delete => Some("\x1b[3~".to_string()),
        Key::Esc => Some("\x1b".to_string()),

        // Ignore the remainder.
        _ => None,
    }
}
