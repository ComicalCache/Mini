use crate::{cursor::Cursor, document::Document, util::CursorStyle};
use std::io::{BufWriter, Error, Stdout, Write};
use termion::{
    clear::All,
    color::{self, Bg, Fg, Reset},
    cursor::{BlinkingBar, BlinkingBlock, Goto, SteadyBlock},
    raw::RawTerminal,
};

const BG: Bg<color::Rgb> = Bg(color::Rgb(41, 44, 51));
const NO_BG: Bg<Reset> = Bg(Reset);
const HIGHLIGHT: Bg<color::Rgb> = Bg(color::Rgb(51, 53, 59));
const NO_HIGHLIGHT: Bg<Reset> = Bg(Reset);
const TXT: Fg<color::Rgb> = Fg(color::Rgb(172, 178, 190));
const NO_TXT: Fg<Reset> = Fg(Reset);

pub struct Viewport {
    pub w: usize,
    pub h: usize,
    pub cur: Cursor,

    pub info_line: String,
    buff: Vec<String>,
    pub cmd: Option<(String, Cursor)>,
}

impl Viewport {
    pub fn new(w: usize, h: usize, x: usize, y: usize) -> Self {
        Viewport {
            w,
            h,
            cur: Cursor::new(x, y),
            info_line: String::with_capacity(w),
            buff: vec![String::with_capacity(w); h - 1],
            cmd: None,
        }
    }

    /// Clears the viewports buffer and sets the cursor to a specified position.
    pub fn clear(&mut self, w: usize, h: usize, x: usize, y: usize) {
        self.w = w;
        self.h = h;
        self.cur = Cursor::new(x, y);

        self.info_line.clear();
        self.buff.resize(h - 1, String::new());
        for line in &mut self.buff {
            line.clear();
        }
        self.cmd = None;
    }

    fn set_line(&mut self, content: &str, offset: usize, line_idx: usize) {
        let lower = offset.saturating_sub(self.cur.x);
        // Only print lines if their content is visible on the screen (horizontal movement).
        if content.chars().count() > lower {
            let upper = (lower + self.w).min(content.chars().count());
            let start = content
                .char_indices()
                .nth(lower)
                .map(|(idx, _)| idx)
                .unwrap();
            let end = content
                .char_indices()
                .nth(upper)
                // Use all remaining bytes if they don't fill the entire line.
                .map_or(content.len(), |(idx, _)| idx);

            self.buff[line_idx].replace_range(0..(end - start), &content[start..end]);
        }
    }

    /// Renders a document to the viewport.
    pub fn render(
        &mut self,
        stdout: &mut BufWriter<RawTerminal<Stdout>>,
        doc: &Document,
        cursor_style: CursorStyle,
    ) -> Result<(), Error> {
        // Make sure that info line is stretched to window width.
        if self.info_line.len() < self.w {
            self.info_line
                .push_str(&" ".repeat(self.w - self.info_line.len()));
        }

        // Calculate which line of text is visible at what line on the screen.
        #[allow(clippy::cast_possible_wrap)]
        let lines_offset = doc.cur.y as isize - self.cur.y as isize;
        for (lines_idx, doc_idx) in (0..self.h - 1).zip(lines_offset + 1..) {
            self.buff[lines_idx] = " ".repeat(self.w);

            // Skip screen lines outside the text line bounds.
            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            if doc_idx < 0 || (doc_idx as usize) >= doc.buff.len() {
                continue;
            }

            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            self.set_line(&doc.buff[doc_idx as usize], doc.cur.x, lines_idx);
        }

        // Set command line and cursor.
        // Can never be larger than u16 since the width is used as upper bound and equal to the terminal size.
        #[allow(clippy::cast_possible_truncation)]
        let cursor = if let Some((cmd_line, cursor)) = &self.cmd {
            // Copy command line and stretch to window width.
            self.buff[self.h - 2].clone_from(cmd_line);
            if cmd_line.len() < self.w {
                self.buff[self.h - 2].push_str(&" ".repeat(self.w - cmd_line.len()));
            }

            Goto(
                (cursor.x as u16).saturating_add(1),
                ((cursor.y + self.h) as u16).saturating_add(1),
            )
        } else {
            Goto(
                (self.cur.x as u16).saturating_add(1),
                (self.cur.y as u16).saturating_add(1),
            )
        };

        // Write the new content.
        write!(
            stdout,
            "{All}{}{HIGHLIGHT}{TXT}{}{NO_HIGHLIGHT}{NO_TXT}",
            Goto(1, 1),
            self.info_line
        )?;
        for (idx, line) in self.buff[..self.buff.len()].iter().enumerate() {
            if idx + 1 == usize::from(cursor.1 - 1) {
                write!(stdout, "\n\r{HIGHLIGHT}{TXT}{line}{NO_HIGHLIGHT}{NO_TXT}")?;
            } else {
                write!(stdout, "\n\r{BG}{TXT}{line}{NO_BG}{NO_TXT}")?;
            }
        }

        match cursor_style {
            CursorStyle::BlinkingBar => write!(stdout, "{cursor}{BlinkingBar}",)?,
            CursorStyle::BlinkingBlock => write!(stdout, "{cursor}{BlinkingBlock}",)?,
            CursorStyle::SteadyBlock => write!(stdout, "{cursor}{SteadyBlock}")?,
        }

        stdout.flush()
    }

    /// Resizes the viewport.
    pub fn resize(&mut self, w: usize, h: usize, x: usize, y: usize) {
        if h != self.h {
            self.buff.resize(h - 1, String::new());
        }

        self.w = w;
        self.h = h;

        self.cur.y = y;
        self.cur.x = x;
    }
}
