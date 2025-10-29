use crate::{cursor::Cursor, document::Document, util::CursorStyle};
use std::io::{BufWriter, Error, Stdout, Write};
use termion::{
    color::{self, Bg, Fg, Reset},
    cursor::{BlinkingBar, BlinkingBlock, Goto, Show, SteadyBlock},
    raw::RawTerminal,
};

/// Background color.
const BG: Bg<color::Rgb> = Bg(color::Rgb(41, 44, 51));
/// Reset background color.
const NO_BG: Bg<Reset> = Bg(Reset);
/// Line highlight background color.
const HIGHLIGHT: Bg<color::Rgb> = Bg(color::Rgb(51, 53, 59));
/// Info line background color.
const INFO: Bg<color::Rgb> = Bg(color::Rgb(59, 61, 66));
/// Selection highlight background color
const SEL: Bg<color::Rgb> = Bg(color::Rgb(75, 78, 87));
/// Text color.
const TXT: Fg<color::Rgb> = Fg(color::Rgb(172, 178, 190));
/// Relative number text color.
const REL_NUMS: Fg<color::Rgb> = Fg(color::Rgb(101, 103, 105));
/// Reset text color.
const NO_TXT: Fg<Reset> = Fg(Reset);

/// The viewport of a (section of a) terminal.
pub struct Viewport {
    /// The total width of the viewport.
    pub w: usize,
    /// The total height of the viewport.
    pub h: usize,
    /// The width of the line number colon.
    pub nums_w: usize,
    /// The width of the buffer content.
    pub buff_w: usize,
    /// The (visible) cursor in the viewport.
    pub cur: Cursor,
    /// If the viewport displays line numbers or not.
    line_nums: bool,
}

impl Viewport {
    pub fn new(w: usize, h: usize, x: usize, y: usize, count: Option<usize>) -> Self {
        let (nums_w, buff_w) = if let Some(count) = count {
            let digits = count.ilog10() as usize + 1;
            (digits + 4, w - digits - 4)
        } else {
            (0, w)
        };

        Viewport {
            w,
            h,
            nums_w,
            buff_w,
            cur: Cursor::new(x, y),
            line_nums: count.is_some(),
        }
    }

    /// Re-initializes the viewport.
    pub fn init(&mut self, w: usize, h: usize, x: usize, y: usize, count: Option<usize>) {
        let (nums_w, buff_w) = if let Some(count) = count {
            let digits = count.ilog10() as usize + 1;
            (digits + 4, w - digits - 4)
        } else {
            (0, w)
        };

        self.w = w;
        self.h = h;
        self.nums_w = nums_w;
        self.buff_w = buff_w;
        self.cur = Cursor::new(x, y);
        self.line_nums = count.is_some();
    }

    /// Sets the absolute line number width.
    pub fn set_number_width(&mut self, n: usize) {
        self.nums_w = n;
        self.buff_w = self.w - n;
    }

    /// Renders a document to the viewport.
    pub fn render_document(
        &mut self,
        stdout: &mut BufWriter<RawTerminal<Stdout>>,
        doc: &Document,
        sel: Option<Cursor>,
    ) -> Result<(), Error> {
        // Update the nums width if the supplied buffer is not correct.
        // log10 + 1 for length + 4 for whitespace and separator.
        if self.line_nums && doc.buff.len().ilog10() as usize + 5 != self.nums_w {
            self.resize(self.w, self.h, Some(doc.buff.len()));
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

        // Calculate which line of text is visible at what line on the screen.
        #[allow(clippy::cast_possible_wrap)]
        let offset = doc.cur.y as isize - self.cur.y as isize;
        // Shifted by one because of info/command line.
        for (idx, doc_idx) in (1..=self.h).zip(offset..) {
            // Set the trailing background color to match if its the cursor line.
            let sel_bg = if idx == self.cur.y + 1 { HIGHLIGHT } else { BG };

            // Set highlight or rel nums.
            if idx == self.cur.y + 1 {
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
                if self.line_nums {
                    write!(stdout, "{}┃", " ".repeat(self.nums_w - 2))?;
                }
                continue;
            }

            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            let doc_idx = doc_idx as usize;

            // Write line numbers.
            if self.line_nums {
                let padding = self.nums_w - 3;
                if doc_idx == doc.cur.y {
                    write!(stdout, "{:>padding$} ┃ ", doc_idx + 1)?;
                } else {
                    write!(stdout, "{:>padding$} ┃ ", doc.cur.y.abs_diff(doc_idx))?;
                }
            }

            // Switch from relative line number color to regular text.
            if idx != self.cur.y + 1 {
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
                                .map_or(content.len(), |(idx, _)| idx)
                        };
                        let sel_end_idx = if end.x >= upper {
                            // Selection started outside of window.
                            end_idx
                        } else {
                            content
                                .char_indices()
                                .nth(end.x)
                                .map_or(content.len(), |(idx, _)| idx)
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
                                .map_or(content.len(), |(idx, _)| idx)
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
                                .map_or(content.len(), |(idx, _)| idx)
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
                if idx == self.cur.y + 1 {
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

        Ok(())
    }

    /// Renders a bar to the viewport.
    pub fn render_bar(
        &self,
        stdout: &mut BufWriter<RawTerminal<Stdout>>,
        doc: &Document,
        prompt: &str,
    ) -> Result<(), Error> {
        let line = &doc.buff[0];
        let w = self.w.saturating_sub(prompt.len());

        let start = doc.cur.x.saturating_sub(self.cur.x);
        let end = (start + w).min(line.chars().count());

        let start_idx = line
            .char_indices()
            .nth(start)
            .map_or(line.len(), |(idx, _)| idx);
        let end_idx = line
            .char_indices()
            .nth(end)
            .map_or(line.len(), |(idx, _)| idx);
        let cmd = &line[start_idx..end_idx];
        let padding = self.w.saturating_sub(prompt.len() + cmd.chars().count());

        write!(
            stdout,
            "{}{INFO}{TXT}{prompt}{cmd}{}{BG}",
            Goto(1, 1),
            " ".repeat(padding)
        )
    }

    pub fn render_cursor(
        &self,
        stdout: &mut BufWriter<RawTerminal<Stdout>>,
        cursor_style: CursorStyle,
        prompt: Option<&str>,
    ) -> Result<(), Error> {
        let cur = if let Some(prompt) = prompt {
            // Plus one because one based.
            // The cursor is bound by the buffer width which is bound by terminal width.
            #[allow(clippy::cast_possible_truncation)]
            Goto(((self.cur.x + prompt.len()) as u16).saturating_add(1), 1)
        } else {
            Goto(
                // Plus one because one based.
                // The cursor is bound by the buffer width which is bound by terminal width.
                #[allow(clippy::cast_possible_truncation)]
                ((self.nums_w + self.cur.x) as u16).saturating_add(1),
                // Plus one because one based, plus one because of the info/command bar.
                // FIXME: this limits the bar to always be exactly two in width.
                // The cursor is bound by the buffer width which is bound by terminal width.
                #[allow(clippy::cast_possible_truncation)]
                (self.cur.y as u16).saturating_add(2),
            )
        };

        // Set cursor.
        match cursor_style {
            CursorStyle::BlinkingBar => write!(stdout, "{cur}{BlinkingBar}{NO_TXT}{NO_BG}{Show}"),
            CursorStyle::BlinkingBlock => {
                write!(stdout, "{cur}{BlinkingBlock}{NO_TXT}{NO_BG}{Show}")
            }
            CursorStyle::SteadyBlock => write!(stdout, "{cur}{SteadyBlock}{NO_TXT}{NO_BG}{Show}"),
        }
    }

    /// Resizes the viewport.
    pub fn resize(&mut self, w: usize, h: usize, count: Option<usize>) {
        let (nums_w, buff_w) = if let Some(count) = count {
            let digits = count.ilog10() as usize + 1;
            (digits + 4, w - digits - 4)
        } else {
            (0, w)
        };

        self.w = w;
        self.h = h;
        self.nums_w = nums_w;
        self.buff_w = buff_w;
        self.line_nums = count.is_some();

        self.cur.x = self.cur.x.min(self.buff_w - 1);
        self.cur.y = self.cur.y.min(self.h - 1);
    }
}
