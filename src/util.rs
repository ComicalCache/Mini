use crate::viewport::{BG, TXT};
use std::{
    fs::{File, OpenOptions},
    io::Error,
    path::Path,
};
use termion::{color, event::Key};

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

/// Converts `Color` from the vt100 crate to termion `Color`.
pub const fn vt100_color_to_rgb(color: vt100::Color, is_fg: bool) -> color::Rgb {
    match color {
        vt100::Color::Default => {
            if is_fg {
                TXT.0
            } else {
                BG.0
            }
        }
        vt100::Color::Idx(i) => {
            match i {
                // 0-15: Standard Atom One Dark Pro Colors.
                0 => color::Rgb(40, 44, 52),         // Black #282c34
                1 | 9 => color::Rgb(224, 108, 117),  // Red #e06c75
                2 | 10 => color::Rgb(152, 195, 121), // Green #98c379
                3 | 11 => color::Rgb(229, 192, 123), // Yellow #e5c07b
                4 | 12 => color::Rgb(97, 175, 239),  // Blue #61afef
                5 | 13 => color::Rgb(198, 120, 221), // Magenta #c678dd
                6 | 14 => color::Rgb(86, 182, 194),  // Cyan #56b6c2
                7 => color::Rgb(171, 178, 191),      // White #abb2bf
                8 => color::Rgb(92, 99, 112),        // Bright Black #5c6370 (Comment color)
                15 => color::Rgb(255, 255, 255),     // Bright White #ffffff

                // 16-231: 6x6x6 Color Cube.
                16..=231 => {
                    let idx = i - 16;
                    let r = (idx / 36) % 6;
                    let g = (idx / 6) % 6;
                    let b = idx % 6;
                    let map = [0, 95, 135, 175, 215, 255];
                    color::Rgb(map[r as usize], map[g as usize], map[b as usize])
                }

                // 232-255: Grayscale Ramp.
                232..=255 => {
                    let gray = 8 + (i - 232) * 10;
                    color::Rgb(gray, gray, gray)
                }
            }
        }
        vt100::Color::Rgb(r, g, b) => color::Rgb(r, g, b),
    }
}

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

        // Common special keys mapped to standard ANSI escape sequences.
        Key::Backspace => Some("\x7f".to_string()),
        Key::Left => Some("\x1b[D".to_string()),
        Key::Right => Some("\x1b[C".to_string()),
        Key::Up => Some("\x1b[A".to_string()),
        Key::Down => Some("\x1b[B".to_string()),
        Key::Delete => Some("\x1b[3~".to_string()),
        Key::Home => Some("\x1b[H".to_string()),
        Key::End => Some("\x1b[F".to_string()),
        Key::PageUp => Some("\x1b[5~".to_string()),
        Key::PageDown => Some("\x1b[6~".to_string()),
        Key::Insert => Some("\x1b[2~".to_string()),
        Key::BackTab => Some("\x1b[Z".to_string()),
        Key::Esc => Some("\x1b".to_string()),

        // Ignore the remaining keys.
        _ => None,
    }
}

/// Converts a `Key` to a String using Application Cursor Keys mode.
pub fn application_key_to_string(key: Key) -> Option<String> {
    match key {
        Key::Up => Some("\x1bOA".to_string()),
        Key::Down => Some("\x1bOB".to_string()),
        Key::Right => Some("\x1bOC".to_string()),
        Key::Left => Some("\x1bOD".to_string()),
        Key::Home => Some("\x1bOH".to_string()),
        Key::End => Some("\x1bOF".to_string()),
        _ => None,
    }
}
