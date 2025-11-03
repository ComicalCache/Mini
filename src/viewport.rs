use crate::{
    cursor::Cursor,
    display::{Cell, Display},
    document::Document,
    util::CursorStyle,
};
use termion::color::{self, Bg, Fg};

/// Background color.
const BG: Bg<color::Rgb> = Bg(color::Rgb(41, 44, 51));
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

/// The viewport of a (section of a) terminal.
pub struct Viewport {
    /// The total width of the viewport.
    pub w: usize,
    /// The total height of the viewport.
    pub h: usize,
    /// The width of the line number colon.
    pub gutter_w: usize,
    /// The width of the buffer content.
    pub buff_w: usize,
    /// The (visible) cursor in the viewport.
    pub cur: Cursor,
    /// If the viewport displays line numbers or not.
    gutter: bool,
}

impl Viewport {
    pub fn new(w: usize, h: usize, x: usize, y: usize, count: Option<usize>) -> Self {
        let (gutter_w, buff_w) = count.map_or((0, w), |count| {
            let digits = count.ilog10() as usize + 1;
            (digits + 4, w - digits - 4)
        });

        Self {
            w,
            h,
            gutter_w,
            buff_w,
            cur: Cursor::new(x, y),
            gutter: count.is_some(),
        }
    }

    /// Sets the absolute line number width.
    pub const fn set_number_width(&mut self, n: usize) {
        self.gutter_w = n;
        self.buff_w = self.w - n;
    }

    /// Renders a document to the `Display`.
    pub fn render_document(&self, display: &mut Display, doc: &Document, sel: Option<Cursor>) {
        // Prepre the selection to be in order if a selection state is active.
        let sel = sel.map(|sel| {
            if doc.cur < sel {
                (doc.cur, sel)
            } else {
                (sel, doc.cur)
            }
        });

        // Calculate which line of text is visible at what line on the screen.
        #[allow(clippy::cast_possible_wrap)]
        let offset = doc.cur.y as isize - self.cur.y as isize;

        // Shifted by one because of info/command line.
        // FIXME: this limits the bar to always be exactly one in height.
        for (y, doc_idx) in (1..=self.h).zip(offset..) {
            let mut x = self.gutter_w;

            // Set base background color depending on if its the cursors line.
            let base_bg = if y == self.cur.y + 1 { HIGHLIGHT } else { BG };
            let base_fg = TXT;

            // Skip screen lines outside the text line bounds.
            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            if doc_idx < 0 || (doc_idx as usize) >= doc.buff.len() {
                for ch in " ".repeat(self.buff_w).chars() {
                    display.update(Cell::new(ch, base_fg, base_bg), x, y);
                    x += 1;
                }
                continue;
            }

            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            let doc_idx = doc_idx as usize;

            let content = &doc.buff[doc_idx];
            let x_offset = doc.cur.x.saturating_sub(self.cur.x);
            let chars = content.chars().skip(x_offset).take(self.buff_w);
            for (idx, mut ch) in chars.enumerate() {
                let mut fg = base_fg;
                let mut bg = base_bg;

                // Layer 1: Selection.
                let char_idx = x_offset + idx;
                if let Some((start, end)) = sel {
                    // Selection on one line and in range.
                    if (start.y == end.y && doc_idx == start.y && char_idx >= start.x && char_idx < end.x)
                    // Start line of selection.
                    || (doc_idx == start.y && start.y < end.y && char_idx >= start.x)
                    // Inbetween lines of selection.
                    || (doc_idx > start.y && doc_idx < end.y)
                    // End line of selection.
                    || (doc_idx == end.y && start.y < end.y && char_idx < end.x)
                    {
                        bg = SEL;
                    }
                }

                // Layer 2: Replace spaces with interdot.
                if ch == ' ' {
                    ch = '·';
                    fg = REL_NUMS;
                }

                // TODO: Layer 3: Syntax Highlighting.

                display.update(Cell::new(ch, fg, bg), x, y);
                x += 1;
            }

            // Add a newline character for visual clarity of trailing whitespaces. Don't show the
            // newline character on the last line of the file.
            let len = content.chars().count();
            let mut true_len = len.saturating_sub(x_offset);
            if doc_idx + 1 != doc.buff.len() && len >= x_offset && true_len < self.buff_w {
                // If part of selection, highlight the newline character for visual clarity.
                if let Some((start, end)) = sel
                    && start.y <= doc_idx
                    && end.y > doc_idx
                {
                    display.update(Cell::new('⏎', REL_NUMS, SEL), x, y);
                } else {
                    display.update(Cell::new('⏎', REL_NUMS, base_bg), x, y);
                }

                x += 1;
                true_len += 1;
            }

            // Stretch current line to end to show highlight properly.
            if true_len < self.buff_w {
                for ch in " ".repeat(self.buff_w - true_len).chars() {
                    display.update(Cell::new(ch, TXT, base_bg), x, y);
                    x += 1;
                }
            }
        }
    }

    /// Renders line numbers to the `Display`.
    pub fn render_gutter(&mut self, display: &mut Display, doc: &Document) {
        if !self.gutter {
            return;
        }

        // Update the nums width if the supplied buffer is not correct.
        // log10 + 1 for length + 4 for whitespace and separator.
        if self.gutter && doc.buff.len().ilog10() as usize + 5 != self.gutter_w {
            self.resize(self.w, self.h, Some(doc.buff.len()));
        }

        // Calculate which line of text is visible at what line on the screen.
        #[allow(clippy::cast_possible_wrap)]
        let offset = doc.cur.y as isize - self.cur.y as isize;

        // Shifted by one because of info/command line.
        // FIXME: this limits the bar to always be exactly one in height.
        for (y, doc_idx) in (1..=self.h).zip(offset..) {
            let mut x = 0;

            // Set base background color and move to the start of the line.
            let (base_bg, base_fg) = if y == self.cur.y + 1 {
                (HIGHLIGHT, TXT)
            } else {
                (BG, REL_NUMS)
            };

            // Skip screen lines outside the text line bounds.
            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            if doc_idx < 0 || (doc_idx as usize) >= doc.buff.len() {
                for ch in format!("{}┃ ", " ".repeat(self.gutter_w - 2)).chars() {
                    display.update(Cell::new(ch, base_fg, base_bg), x, y);
                    x += 1;
                }
                continue;
            }

            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            let doc_idx = doc_idx as usize;

            // Write line numbers.
            let padding = self.gutter_w - 3;
            if doc_idx == doc.cur.y {
                for ch in format!("{:>padding$} ┃ ", doc_idx + 1).chars() {
                    display.update(Cell::new(ch, base_fg, base_bg), x, y);
                    x += 1;
                }
            } else {
                for ch in format!("{:>padding$} ┃ ", doc.cur.y.abs_diff(doc_idx)).chars() {
                    display.update(Cell::new(ch, base_fg, base_bg), x, y);
                    x += 1;
                }
            }
        }
    }

    /// Renders a bar to the `Display`.
    pub fn render_bar(&self, display: &mut Display, doc: &Document, prompt: &str) {
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

        for (x, ch) in format!("{prompt}{cmd}{}", " ".repeat(padding))
            .chars()
            .enumerate()
        {
            display.update(Cell::new(ch, TXT, INFO), x, 0);
        }
    }

    /// Renders a `Cursor` to the `Display`.
    pub fn render_cursor(
        &self,
        display: &mut Display,
        cursor_style: CursorStyle,
        prompt: Option<&str>,
    ) {
        // The cursor is bound by the buffer width which is bound by terminal width.
        #[allow(clippy::cast_possible_truncation)]
        let cur = prompt.map_or_else(
            || {
                Cursor::new(
                    self.gutter_w + self.cur.x,
                    // Plus  one because of the info/command bar.
                    // FIXME: this limits the bar to always be exactly one in height.
                    self.cur.y + 1,
                )
            },
            |prompt| Cursor::new(self.cur.x + prompt.len(), 0),
        );

        display.set_cursor(cur, cursor_style);
    }

    /// Resizes the viewport.
    pub fn resize(&mut self, w: usize, h: usize, count: Option<usize>) {
        let (gutter_w, buff_w) = count.map_or((0, w), |count| {
            let digits = count.ilog10() as usize + 1;
            (digits + 4, w - digits - 4)
        });

        self.w = w;
        self.h = h;
        self.gutter_w = gutter_w;
        self.buff_w = buff_w;
        self.gutter = count.is_some();

        self.cur.x = self.cur.x.min(self.buff_w - 1);
        self.cur.y = self.cur.y.min(self.h - 1);
    }
}
