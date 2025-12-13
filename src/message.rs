use crate::util::TAB_WIDTH;
use std::{fmt::Display, str::Lines};
use unicode_width::UnicodeWidthChar;

/// Kind of the message.
#[derive(Clone)]
pub enum MessageKind {
    Info,
    Error,
}

/// A message to be displayed to the user to convey information or show errors.
#[derive(Clone)]
pub struct Message {
    pub kind: MessageKind,

    /// The text of the message.
    pub text: String,
    /// Amount of lines.
    pub lines: usize,

    /// Scrolling offset for long messages.
    pub scroll: usize,
}

impl Message {
    pub fn new(kind: MessageKind, text: String, width: usize) -> Self {
        let mut ret = Self {
            kind,
            text,
            lines: 0,
            scroll: 0,
        };
        ret.lines = ret.iter(width).count();

        ret
    }

    /// Returns an iterator over the visual lines of the message, wrapped to `width`.
    pub fn iter(&self, width: usize) -> MessageIter<'_> {
        MessageIter {
            lines: self.text.lines(),
            current_line: None,
            width,
        }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            MessageKind::Info => writeln!(f, "Info:")?,
            MessageKind::Error => writeln!(f, "Error:")?,
        }
        write!(f, "{}", self.text)
    }
}

/// An iterator that yields wrapped lines for a message.
pub struct MessageIter<'a> {
    /// Iterator over the logical lines of the text.
    lines: Lines<'a>,
    /// The remainder of the currently wrapped logical line.
    current_line: Option<&'a str>,
    /// The target visual width.
    width: usize,
}

impl<'a> Iterator for MessageIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        // Get the remainder of the current line or the next logical line.
        let text = if let Some(text) = self.current_line {
            text
        } else {
            let next_line = self.lines.next()?;
            if next_line.is_empty() {
                return Some("");
            }
            next_line
        };

        // Calculate how much text fits into self.width.
        let mut width = 0;
        let mut split_idx = text.len();

        for (idx, ch) in text.char_indices() {
            let ch_width = if ch == '\t' {
                TAB_WIDTH - (width % TAB_WIDTH)
            } else {
                ch.width().unwrap_or(0)
            };

            if ch_width == 0 {
                continue;
            }

            // If the character doesn't fit, split the line.
            if width + ch_width > self.width {
                // If the very first character is already too wide for the entire line panic.
                assert!(idx != 0);
                split_idx = idx;
                break;
            }

            width += ch_width;
        }

        let (chunk, rest) = text.split_at(split_idx);

        if rest.is_empty() {
            self.current_line = None;
        } else {
            self.current_line = Some(rest);
        }

        Some(chunk)
    }
}
