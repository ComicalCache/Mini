use crate::{cursor::Cursor, document::Document, util::CursorStyle};
use std::io::{BufWriter, Error, Stdout, Write};
use termion::{
    clear::All,
    color::{self, Bg, Fg, Reset},
    cursor::{BlinkingBar, BlinkingBlock, Goto, Hide, Show, SteadyBlock},
    raw::RawTerminal,
};

const BG: Bg<color::Rgb> = Bg(color::Rgb(41, 44, 51));
const NO_BG: Bg<Reset> = Bg(Reset);
const HIGHLIGHT: Bg<color::Rgb> = Bg(color::Rgb(51, 53, 59));
const TXT: Fg<color::Rgb> = Fg(color::Rgb(172, 178, 190));
const REL_NUMS: Fg<color::Rgb> = Fg(color::Rgb(101, 103, 105));
const NO_TXT: Fg<Reset> = Fg(Reset);

pub struct Viewport {
    w: usize,
    h: usize,
    nums_w: usize,
    nums_h: usize,
    pub buff_w: usize,
    pub buff_h: usize,
    pub cur: Cursor,

    pub info_line: String,
    nums_buff: Vec<String>,
    buff: Vec<String>,
    pub cmd: Option<(String, Cursor)>,
}

impl Viewport {
    pub fn new(w: usize, h: usize, x: usize, y: usize, count: usize) -> Self {
        let digits = count.ilog10() as usize + 1;
        Viewport {
            w: w,
            h: h,
            nums_w: digits + 4,
            nums_h: h,
            buff_w: w - digits - 4,
            buff_h: h,
            cur: Cursor::new(x, y),
            info_line: String::with_capacity(w),
            nums_buff: vec![String::with_capacity(digits + 4); h - 1],
            buff: vec![String::with_capacity(w - digits - 4); h - 1],
            cmd: None,
        }
    }

    /// Clears the viewports buffer and sets the cursor to a specified position.
    pub fn clear(&mut self, w: usize, h: usize, x: usize, y: usize, count: usize) {
        let digits = count.ilog10() as usize + 1;

        self.w = w;
        self.h = h;
        self.nums_w = digits + 4;
        self.nums_h = h;
        self.buff_w = w - digits - 4;
        self.buff_h = h;
        self.cur = Cursor::new(x, y);

        self.info_line.clear();
        self.nums_buff
            .resize(h - 1, String::with_capacity(digits + 4));
        self.buff
            .resize(h - 1, String::with_capacity(w - digits - 4));
        for idx in 0..h - 1 {
            self.nums_buff[idx].clear();
            self.buff[idx].clear();
        }
        self.cmd = None;
    }

    /// Renders a document to the viewport.
    pub fn render(
        &mut self,
        stdout: &mut BufWriter<RawTerminal<Stdout>>,
        doc: &Document,
        cursor_style: CursorStyle,
    ) -> Result<(), Error> {
        // Update the nums width if the supplied buffer is not correct.
        // Avoid leaking variables into the scope.
        {
            let digits = doc.buff.len().ilog10() as usize + 1;
            if digits + 4 != self.nums_w {
                self.resize(self.w, self.h, self.cur.x, self.cur.y, doc.buff.len());
            }
        }

        // Make sure that info line is stretched to window width.
        if self.info_line.chars().count() < self.w {
            self.info_line
                .push_str(&" ".repeat(self.w - self.info_line.chars().count()));
        }

        // Calculate which line of text is visible at what line on the screen.
        #[allow(clippy::cast_possible_wrap)]
        let lines_offset = doc.cur.y as isize - self.cur.y as isize;
        for (lines_idx, doc_idx) in (0..self.buff_h - 1).zip(lines_offset + 1..) {
            self.buff[lines_idx].clear();

            // Skip screen lines outside the text line bounds.
            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            if doc_idx < 0 || (doc_idx as usize) >= doc.buff.len() {
                self.nums_buff[lines_idx] = format!("{}┃ ", " ".repeat(self.nums_w - 2));
                continue;
            }

            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            {
                let padding = self.nums_w - 3;
                if doc_idx as usize == doc.cur.y {
                    self.nums_buff[lines_idx] = format!("{:>padding$} ┃ ", doc_idx + 1);
                } else {
                    self.nums_buff[lines_idx] =
                        format!("{:>padding$} ┃ ", doc.cur.y.abs_diff(doc_idx as usize));
                }
            }

            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            let content = &doc.buff[doc_idx as usize];
            let lower = doc.cur.x.saturating_sub(self.cur.x);
            // Only print lines if their content is visible on the screen (horizontal movement).
            if content.chars().count() > lower {
                let upper = (lower + self.buff_w).min(content.chars().count());
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

                self.buff[lines_idx].replace_range(.., &content[start..end]);
            }
        }

        // Set command line and cursor.
        // Can never be larger than u16 since the width is used as upper bound and equal to the terminal size.
        #[allow(clippy::cast_possible_truncation)]
        let cur = if let Some((cmd_line, cur)) = &self.cmd {
            // Copy command line and stretch to buffer window width.
            self.buff[self.buff_h - 2].clone_from(cmd_line);
            if cmd_line.chars().count() < self.buff_w {
                self.buff[self.buff_h - 2]
                    .push_str(&" ".repeat(self.buff_w - cmd_line.chars().count()));
            }

            Goto(
                ((self.nums_w + cur.x) as u16).saturating_add(1),
                ((cur.y + self.buff_h) as u16).saturating_add(1),
            )
        } else {
            Goto(
                ((self.nums_w + self.cur.x) as u16).saturating_add(1),
                (self.cur.y as u16).saturating_add(1),
            )
        };

        // Write the new content.
        write!(
            stdout,
            "{All}{Hide}{}{HIGHLIGHT}{TXT}{}{BG}",
            Goto(1, 1),
            self.info_line
        )?;
        for (idx, (line, nums)) in self.buff[..self.buff.len()]
            .iter()
            .zip(&self.nums_buff[..self.nums_buff.len()])
            .enumerate()
        {
            if idx + 1 == usize::from(cur.1 - 1) {
                // Fill the cursor line with spaces so the highlight is shown.
                write!(
                    stdout,
                    "\n\r{HIGHLIGHT}{nums}{line}{}{BG}",
                    " ".repeat(self.buff_w - line.chars().count())
                )?;
            } else {
                write!(stdout, "\n\r{REL_NUMS}{nums}{TXT}{line}")?;
            }
        }

        match cursor_style {
            CursorStyle::BlinkingBar => write!(stdout, "{cur}{BlinkingBar}{NO_TXT}{NO_BG}{Show}",)?,
            CursorStyle::BlinkingBlock => {
                write!(stdout, "{cur}{BlinkingBlock}{NO_TXT}{NO_BG}{Show}",)?;
            }
            CursorStyle::SteadyBlock => write!(stdout, "{cur}{SteadyBlock}{NO_TXT}{NO_BG}{Show}")?,
        }

        stdout.flush()
    }

    /// Resizes the viewport.
    pub fn resize(&mut self, w: usize, h: usize, x: usize, y: usize, count: usize) {
        let digits = count.ilog10() as usize + 1;

        if h != self.buff_h {
            self.nums_buff
                .resize(h - 1, String::with_capacity(digits + 4));
            self.buff
                .resize(h - 1, String::with_capacity(w - digits - 4));
        }

        self.w = w;
        self.h = h;
        self.nums_w = digits + 4;
        self.nums_h = h;
        self.buff_w = w - digits - 4;
        self.buff_h = h;

        self.cur.y = y;
        self.cur.x = x;
    }
}
