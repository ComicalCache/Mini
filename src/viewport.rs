use crate::{
    cursor::Cursor,
    display::{Cell, Display},
    document::Document,
    util::CursorStyle,
};
use termion::color::{self, Bg, Fg};
use tree_sitter_highlight::HighlightEvent;

/// Background color.
const BG: Bg<color::Rgb> = Bg(color::Rgb(41, 44, 51));
/// Line highlight background color.
const HIGHLIGHT: Bg<color::Rgb> = Bg(color::Rgb(51, 53, 59));
/// Info line background color.
const INFO: Bg<color::Rgb> = Bg(color::Rgb(59, 61, 66));
/// Selection highlight background color
const SEL: Bg<color::Rgb> = Bg(color::Rgb(75, 78, 87));
/// Text color.
pub const TXT: Fg<color::Rgb> = Fg(color::Rgb(172, 178, 190));
/// Relative number text color.
const REL_NUMS: Fg<color::Rgb> = Fg(color::Rgb(101, 103, 105));
/// Whitespace symbol text color.
const WHITESPACE: Fg<color::Rgb> = Fg(color::Rgb(68, 71, 79));
/// Background to warn of tab characters.
const CHAR_WARN: Bg<color::Rgb> = Bg(color::Rgb(181, 59, 59));

/// The viewport of a (section of a) `Display`.
pub struct Viewport {
    /// The total width of the viewport.
    pub w: usize,
    /// The total height of the viewport.
    pub h: usize,
    /// The physical x offset of the viewport on the `Display`.
    pub x_off: usize,
    /// The physical y offset of the viewport on the `Display`.
    pub y_off: usize,
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
    pub fn new(
        w: usize,
        h: usize,
        x: usize,
        y: usize,
        x_off: usize,
        y_off: usize,
        count: Option<usize>,
    ) -> Self {
        let (gutter_w, buff_w) = count.map_or((0, w), |count| {
            let digits = count.ilog10() as usize + 1;
            (digits + 4, w - digits - 4)
        });

        Self {
            w,
            h,
            x_off,
            y_off,
            gutter_w,
            buff_w,
            cur: Cursor::new(x, y),
            gutter: count.is_some(),
        }
    }

    /// Resizes the viewport.
    pub fn resize(&mut self, w: usize, h: usize, x_off: usize, y_off: usize, count: Option<usize>) {
        let (gutter_w, buff_w) = count.map_or((0, w), |count| {
            let digits = count.ilog10() as usize + 1;
            (digits + 4, w - digits - 4)
        });

        self.w = w;
        self.h = h;
        self.x_off = x_off;
        self.y_off = y_off;
        self.gutter_w = gutter_w;
        self.buff_w = buff_w;
        self.gutter = count.is_some();

        self.cur.x = self.cur.x.min(self.buff_w - 1);
        self.cur.y = self.cur.y.min(self.h - 1);
    }

    /// Sets the gutter width.
    pub const fn set_gutter_width(&mut self, n: usize) {
        self.gutter_w = n + 4;
        self.buff_w = self.w - n - 4;
    }

    /// Sets the absolute gutter width.
    pub const fn set_absolute_gutter_width(&mut self, n: usize) {
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
        let y_off = doc.cur.y - self.cur.y;
        let y_max = y_off + self.h;
        // Calculate the offset of characters on the screen.
        let x_off = doc.cur.x - self.cur.x;
        let x_max = x_off + self.buff_w;

        let mut color_stack: Vec<Fg<color::Rgb>> = Vec::new();
        let mut y = 0;
        let mut x = 0;

        'event_loop: for event in &doc.highlighter.highlights {
            match event {
                HighlightEvent::HighlightStart(h) => match doc.highlighter.names.get(h.0) {
                    Some(name) => {
                        color_stack.push(*doc.highlighter.colors.get(name).unwrap_or(&TXT));
                    }
                    None => color_stack.push(TXT),
                },
                HighlightEvent::HighlightEnd => {
                    color_stack.pop();
                }
                HighlightEvent::Source { start, end } => {
                    for ch in doc.contiguous_buff[*start..*end].chars() {
                        if y >= y_off && y < y_max && x >= x_off && x < x_max {
                            let mut display_ch = ch;
                            let mut fg = *color_stack.last().unwrap_or(&TXT);
                            let mut bg = if y == doc.cur.y { HIGHLIGHT } else { BG };

                            let screen_y = y - y_off + self.y_off;
                            let screen_x = self.gutter_w + x - x_off + self.x_off;

                            // Layer 1: Selection.
                            if let Some((start, end)) = sel {
                                let pos = Cursor::new(x, y);
                                if pos >= start && pos < end {
                                    bg = SEL;
                                }
                            }

                            // Layer 2: Character replacement.
                            if display_ch == ' ' {
                                display_ch = '·';
                                fg = WHITESPACE;
                            }
                            if display_ch == '\n' {
                                display_ch = '⏎';
                                fg = WHITESPACE;
                            }
                            if display_ch == '\r' {
                                display_ch = '↤';
                                fg = TXT;
                                bg = CHAR_WARN;
                            }
                            if display_ch == '\t' {
                                display_ch = '↦';
                                fg = TXT;
                                bg = CHAR_WARN;
                            }

                            display.update(Cell::new(display_ch, fg, bg), screen_x, screen_y);
                        }

                        if ch == '\n' {
                            y += 1;
                            x = 0;
                        } else {
                            x += 1;
                        }

                        if y >= y_max {
                            break 'event_loop;
                        }
                    }
                }
            }
        }

        // Render trailing whitespace to override previous screen content. The previous loop only renders the current
        // content without regard of removing existing content, which is why this second render pass is necessary.
        for (y, doc_idx) in (0..self.h).zip(y_off..) {
            // Set base background color depending on if its the cursors line.
            let base_bg = if y == self.cur.y { HIGHLIGHT } else { BG };

            // Skip screen lines outside the text line bounds.
            if doc_idx >= doc.buff.len() {
                for x in self.gutter_w..self.w {
                    display.update(Cell::new(' ', TXT, base_bg), x + self.x_off, y + self.y_off);
                }
                continue;
            }

            let len = doc.line_count(doc_idx).unwrap();
            // Calculate the end of the line contents.
            let x = self.gutter_w + (len.saturating_sub(x_off)) + self.x_off;
            // Stretch current line to end to show highlight properly.
            for x in x..self.w {
                display.update(Cell::new(' ', TXT, base_bg), x, y + self.y_off);
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
            self.resize(self.w, self.h, self.x_off, self.y_off, Some(doc.buff.len()));
        }

        // Calculate which line of text is visible at what line on the screen.
        #[allow(clippy::cast_possible_wrap)]
        let offset = doc.cur.y as isize - self.cur.y as isize;
        for (y, doc_idx) in (0..self.h).zip(offset..) {
            let mut x = self.x_off;

            // Set base background color and move to the start of the line.
            let (base_bg, base_fg) = if y == self.cur.y {
                (HIGHLIGHT, TXT)
            } else {
                (BG, REL_NUMS)
            };

            // Skip screen lines outside the text line bounds.
            // The value is guaranteed positive at that point.
            #[allow(clippy::cast_sign_loss)]
            if doc_idx < 0 || (doc_idx as usize) >= doc.buff.len() {
                for ch in format!("{}┃ ", " ".repeat(self.gutter_w - 2)).chars() {
                    display.update(Cell::new(ch, base_fg, base_bg), x, y + self.y_off);
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
                    display.update(Cell::new(ch, base_fg, base_bg), x, y + self.y_off);
                    x += 1;
                }
            } else {
                for ch in format!("{:>padding$} ┃ ", doc.cur.y.abs_diff(doc_idx)).chars() {
                    display.update(Cell::new(ch, base_fg, base_bg), x, y + self.y_off);
                    x += 1;
                }
            }
        }
    }

    /// Renders a bar to the `Display`.
    pub fn render_bar(
        &self,
        line: &str,
        y: usize,
        display: &mut Display,
        doc: &Document,
        prompt: &str,
    ) {
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
            display.update(Cell::new(ch, TXT, INFO), x + self.x_off, y + self.y_off);
        }
    }

    /// Renders a `Cursor` to the `Display`.
    pub fn render_cursor(
        &self,
        display: &mut Display,
        cursor_style: CursorStyle,
        off: Option<usize>,
    ) {
        // The cursor is bound by the buffer width which is bound by terminal width.
        #[allow(clippy::cast_possible_truncation)]
        let cur = off.map_or_else(
            || {
                Cursor::new(
                    self.gutter_w + self.cur.x + self.x_off,
                    self.cur.y + self.y_off,
                )
            },
            |off| Cursor::new(self.cur.x + off + self.x_off, self.y_off),
        );

        display.set_cursor(cur, cursor_style);
    }
}
