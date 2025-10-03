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
const INFO: Bg<color::Rgb> = Bg(color::Rgb(59, 61, 66));
const SEL: Bg<color::Rgb> = Bg(color::Rgb(75, 78, 87));
const TXT: Fg<color::Rgb> = Fg(color::Rgb(172, 178, 190));
const REL_NUMS: Fg<color::Rgb> = Fg(color::Rgb(101, 103, 105));
const NO_TXT: Fg<Reset> = Fg(Reset);

pub struct Viewport {
    pub w: usize,
    pub h: usize,
    pub nums_w: usize,
    pub buff_w: usize,
    pub cur: Cursor,

    pub info_line: String,
    pub cmd: Option<(String, Cursor)>,
}

impl Viewport {
    pub fn new(w: usize, h: usize, x: usize, y: usize, count: usize) -> Self {
        let digits = count.ilog10() as usize + 1;
        Viewport {
            w,
            h,
            nums_w: digits + 4,
            buff_w: w - digits - 4,
            cur: Cursor::new(x, y),
            info_line: String::with_capacity(w),
            cmd: None,
        }
    }

    pub fn init(&mut self, w: usize, h: usize, x: usize, y: usize, count: usize) {
        let digits = count.ilog10() as usize + 1;

        self.w = w;
        self.h = h;
        self.nums_w = digits + 4;
        self.buff_w = w - digits - 4;
        self.cur = Cursor::new(x, y);

        self.info_line.clear();
        self.cmd = None;
    }

    /// Renders a document to the viewport.
    pub fn render(
        &mut self,
        stdout: &mut BufWriter<RawTerminal<Stdout>>,
        doc: &Document,
        sel: Option<Cursor>,
        cursor_style: CursorStyle,
    ) -> Result<(), Error> {
        // Update the nums width if the supplied buffer is not correct.
        // Avoid leaking variables into the scope.
        {
            let digits = doc.buff.len().ilog10() as usize + 1;
            if digits + 4 != self.nums_w {
                self.resize(self.w, self.h, doc.buff.len());
            }
        }

        // Prepre the selection to be in order if a selection state is active.
        let sel = if let Some(sel) = sel {
            if doc.cur < sel {
                Some((doc.cur, sel))
            } else {
                Some((sel, doc.cur))
            }
        } else {
            None
        };

        // Make sure that info line is stretched to window width.
        if self.info_line.chars().count() < self.w {
            self.info_line
                .push_str(&" ".repeat(self.w - self.info_line.chars().count()));
        }
        write!(
            stdout,
            "{All}{Hide}{}{INFO}{TXT}{}{BG}",
            Goto(1, 1),
            self.info_line
        )?;

        // Get cursor position. Add two to the y coordinate because one based and info line at the top.
        // Can never be larger than u16 since the width is used as upper bound and equal to the terminal size.
        #[allow(clippy::cast_possible_truncation)]
        let cur = if let Some((_, cur)) = &self.cmd {
            Goto(
                ((self.nums_w + cur.x) as u16).saturating_add(1),
                ((cur.y + self.h) as u16).saturating_add(2),
            )
        } else {
            Goto(
                ((self.nums_w + self.cur.x) as u16).saturating_add(1),
                (self.cur.y as u16).saturating_add(2),
            )
        };

        // Calculate which line of text is visible at what line on the screen.
        #[allow(clippy::cast_possible_wrap)]
        let lines_offset = doc.cur.y as isize - self.cur.y as isize;
        for (idx, doc_idx) in (1..self.h).zip(lines_offset..) {
            // Set the trailing background color to match if its the cursor line.
            let sel_bg = if idx == usize::from(cur.1 - 1) {
                HIGHLIGHT
            } else {
                BG
            };

            // Set highlight or rel nums.
            if idx == usize::from(cur.1 - 1) {
                // The idx is bound by the height which is bound by u16 when passed by the terminal.
                #[allow(clippy::cast_possible_truncation)]
                write!(stdout, "{}{HIGHLIGHT}{TXT}", Goto(1, idx as u16 + 1))?;
            } else {
                // The idx is bound by the height which is bound by u16 when passed by the terminal.
                #[allow(clippy::cast_possible_truncation)]
                write!(stdout, "{}{REL_NUMS}", Goto(1, idx as u16 + 1))?;
            }

            // Skip screen lines outside the text line bounds.
            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            if doc_idx < 0 || (doc_idx as usize) >= doc.buff.len() {
                write!(stdout, "{}┃", " ".repeat(self.nums_w - 2))?;
                continue;
            }

            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            let doc_idx = doc_idx as usize;

            // Write line numbers.
            {
                let padding = self.nums_w - 3;
                if doc_idx == doc.cur.y {
                    write!(stdout, "{:>padding$} ┃ ", doc_idx + 1)?;
                } else {
                    write!(stdout, "{:>padding$} ┃ ", doc.cur.y.abs_diff(doc_idx))?;
                }
            }

            // Switch from relative line number color to regular text.
            if idx != usize::from(cur.1 - 1) {
                write!(stdout, "{TXT}")?;
            }

            let content = &doc.buff[doc_idx];
            let lower = doc.cur.x.saturating_sub(self.cur.x);
            // Only print lines if their content is visible on the screen (horizontal movement).
            if content.chars().count() > lower {
                let upper = (lower + self.buff_w).min(content.chars().count());
                let start_idx = content
                    .char_indices()
                    .nth(lower)
                    .map_or(content.len(), |(idx, _)| idx);
                let end_idx = content
                    .char_indices()
                    .nth(upper)
                    // Use all remaining bytes if they don't fill the entire line.
                    .map_or(content.len(), |(idx, _)| idx);

                match sel {
                    // Empty selection
                    Some((start, end)) if start == end => {
                        write!(stdout, "{}", &content[start_idx..end_idx])?;
                    }
                    // Selection on one line.
                    Some((start, end)) if start.y == end.y && start.y == doc_idx => {
                        let sel_start_idx = if start.x <= lower {
                            // Selection started outside of visible line.
                            start_idx
                        } else {
                            // Find selection start index.
                            content
                                .char_indices()
                                .nth(start.x)
                                .map(|(idx, _)| idx)
                                .expect("Illegal state")
                        };
                        let sel_end_idx = if end.x >= upper {
                            // Selection started outside of window.
                            end_idx
                        } else {
                            content
                                .char_indices()
                                .nth(end.x)
                                .map(|(idx, _)| idx)
                                .expect("Illegal state")
                        };

                        if sel_start_idx > start_idx {
                            write!(stdout, "{}", &content[start_idx..sel_start_idx])?;
                        }
                        write!(stdout, "{SEL}{}", &content[sel_start_idx..sel_end_idx])?;
                        if sel_end_idx < end_idx {
                            write!(stdout, "{sel_bg}{}", &content[sel_end_idx..end_idx])?;
                        } else {
                            write!(stdout, "{sel_bg}")?;
                        }
                    }
                    // Start line of selection
                    Some((start, _)) if start.y == doc_idx => {
                        let sel_start_idx = if start.x <= lower {
                            // Selection started outside of visible line.
                            start_idx
                        } else {
                            // Find selection start index.
                            content
                                .char_indices()
                                .nth(start.x)
                                .map(|(idx, _)| idx)
                                .expect("Illegal state")
                        };

                        if sel_start_idx > start_idx {
                            write!(stdout, "{}", &content[start_idx..sel_start_idx])?;
                        }
                        write!(stdout, "{SEL}{}{sel_bg}", &content[sel_start_idx..end_idx],)?;
                    }
                    // Inbetween lines of selection
                    Some((start, end)) if start.y < doc_idx && doc_idx < end.y => {
                        write!(stdout, "{SEL}{}{sel_bg}", &content[start_idx..end_idx],)?;
                    }
                    // End line of selection
                    Some((_, end)) if end.y == doc_idx => {
                        let sel_end_idx = if end.x >= upper {
                            // Selection started outside of window.
                            end_idx
                        } else {
                            content
                                .char_indices()
                                .nth(end.x)
                                .map(|(idx, _)| idx)
                                .expect("Illegal state")
                        };

                        write!(stdout, "{SEL}{}", &content[start_idx..sel_end_idx])?;
                        if sel_end_idx < end_idx {
                            write!(stdout, "{sel_bg}{}", &content[sel_end_idx..end_idx])?;
                        } else {
                            write!(stdout, "{sel_bg}")?;
                        }
                    }
                    _ => write!(stdout, "{}", &content[start_idx..end_idx])?,
                }

                // Stretch current line to end to show highlight properly.
                if idx == usize::from(cur.1 - 1) {
                    write!(
                        stdout,
                        "{}{BG}",
                        " ".repeat(self.buff_w - content[start_idx..end_idx].chars().count())
                    )?;
                }
            } else {
                // Stretch current line to end to show highlight properly.
                write!(stdout, "{}{BG}", " ".repeat(self.buff_w))?;
            }
        }

        // Set command line.
        // Can never be larger than u16 since the width is used as upper bound and equal to the terminal size.
        #[allow(clippy::cast_possible_truncation)]
        if let Some((cmd_line, _)) = &self.cmd {
            write!(
                stdout,
                "{INFO}{}{REL_NUMS}{}> {TXT}{cmd_line}{}",
                Goto(1, self.h as u16),
                " ".repeat(self.nums_w - 2),
                " ".repeat(self.buff_w - cmd_line.chars().count())
            )?;
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
    pub fn resize(&mut self, w: usize, h: usize, count: usize) {
        let digits = count.ilog10() as usize + 1;

        self.w = w;
        self.h = h;
        self.nums_w = digits + 4;
        self.buff_w = w - digits - 4;

        self.cur.x = self.cur.x.min(self.buff_w - 1);
        // Minus two because one for zero based, one for the info line.
        self.cur.y = self.cur.y.min(self.h - 2);
    }
}
