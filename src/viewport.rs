use crate::{
    cursor::{Cursor, CursorStyle},
    display::{Cell, Display, PLACEHOLDER},
    document::Document,
    message::{Message, MessageKind},
    selection::Selection,
    shell_command::util::vt100_color_to_rgb,
    util::{TAB_WIDTH, text_width},
};
use termion::color::{self, Bg, Fg};
use unicode_width::UnicodeWidthChar;
use vt100::Parser;

#[macro_export]
/// Convenience macro for calling movement functions. Expects a `BaseBuffer` as member `base`.
macro_rules! shift {
    ($self:ident, $func:ident) => {{
        $self.base.doc_view.$func(&mut $self.base.doc, 1);
        $self.base.update_selection();
    }};
}

/// Background color.
pub const BG: Bg<color::Rgb> = Bg(color::Rgb(41, 44, 51));
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
/// Info message text color.
const INFO_TXT: Fg<color::Rgb> = Fg(color::Rgb(55, 131, 181));
/// Error message text color.
const ERROR_TXT: Fg<color::Rgb> = Fg(color::Rgb(181, 59, 59));

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
    /// The scroll x offset of the document within the viewport.
    pub scroll_x: usize,
    /// The scroll y offset of the document within the viewport.
    pub scroll_y: usize,
    /// The width of the line number colon.
    pub gutter_w: usize,
    /// The width of the buffer content.
    pub buff_w: usize,
    /// If the viewport displays line numbers or not.
    gutter: bool,
}

impl Viewport {
    pub fn new(w: usize, h: usize, x_off: usize, y_off: usize, count: Option<usize>) -> Self {
        let (gutter_w, buff_w) = count.map_or((0, w), |count| {
            let digits = count.ilog10() as usize + 1;
            (digits + 4, w - digits - 4)
        });

        Self {
            w,
            h,
            x_off,
            y_off,
            scroll_x: 0,
            scroll_y: 0,
            gutter_w,
            buff_w,
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
    }

    pub fn recalculate_viewport(&mut self, doc: &Document) {
        let line = doc
            .line(doc.cur.y)
            .map(|l| l.to_string())
            .unwrap_or_default();
        let visual_x = text_width(&line, doc.cur.x);

        self.scroll_x = self
            .scroll_x
            .clamp(visual_x.saturating_sub(self.buff_w - 1), visual_x);
        self.scroll_y = self
            .scroll_y
            .clamp(doc.cur.y.saturating_sub(self.h - 1), doc.cur.y);
    }

    /// Sets the gutter width.
    pub const fn set_gutter_width(&mut self, n: usize) {
        self.gutter_w = n + 4;
        self.buff_w = self.w - n - 4;
    }

    /// Renders a message overlay to the `Display`. Should be called after `render_document` because it will get
    /// overwritten otherwise. This function assumes that `MessageIter` correctly calculates the lines and does
    /// NO bounds-checking when updating the display.
    pub fn render_message(&self, display: &mut Display, message: &Message) {
        let count = (message.lines.saturating_sub(message.scroll)).min(self.h / 3);

        let lines = message.iter(self.w).skip(message.scroll).take(count);
        for (y, line) in lines.enumerate() {
            let mut x = 0;
            let display_y = self.y_off + y;

            for ch in line.chars() {
                let mut fg = match message.kind {
                    MessageKind::Info => INFO_TXT,
                    MessageKind::Error => ERROR_TXT,
                };
                let mut bg = INFO;

                // Layer 1: Character replacement.
                let display_ch = match ch {
                    '\r' => {
                        fg = TXT;
                        bg = CHAR_WARN;
                        '↤'
                    }
                    '\t' => {
                        fg = TXT;
                        bg = CHAR_WARN;
                        '↦'
                    }
                    _ => ch,
                };

                let width = match ch {
                    '\r' => 1,
                    '\t' => TAB_WIDTH - (x % TAB_WIDTH),
                    ch => ch.width().unwrap_or(0),
                };
                if width == 0 {
                    continue;
                }

                // Assert that MessageIter correctly calculates lines.
                assert!(x + width <= self.w);
                display.update(Cell::new(display_ch, fg, bg), self.x_off + x, display_y);

                // Layer 2: Expand tabs.
                if ch == '\t' {
                    // Write as many spaces as needed after the tab character.
                    for n in 1..=width {
                        display.update(Cell::new(' ', fg, bg), self.x_off + x + n, display_y);
                    }
                } else {
                    // Mark all following cells of wide characters as taken.
                    for n in 1..width {
                        display.update(
                            Cell::new(PLACEHOLDER, fg, bg),
                            self.x_off + x + n,
                            display_y,
                        );
                    }
                }

                x += width;
            }

            // Clear the rest of the line
            while x < self.w {
                display.update(Cell::new(' ', ERROR_TXT, INFO), self.x_off + x, display_y);
                x += 1;
            }
        }
    }

    /// Renders a document to the `Display`.
    pub fn render_document(
        &self,
        display: &mut Display,
        doc: &Document,
        selections: &Vec<Selection>,
    ) {
        for y in 0..self.h {
            let doc_y = self.scroll_y + y;
            let mut x = 0;

            // Draw the contents of the line.
            if let Some(line) = doc.line(doc_y) {
                for (idx, ch) in line.chars().enumerate() {
                    let mut fg = TXT;
                    let mut bg = if doc_y == doc.cur.y { HIGHLIGHT } else { BG };

                    // Layer 1: Character replacement.
                    let mut display_ch = ch;
                    match ch {
                        ' ' => {
                            display_ch = '·';
                            fg = WHITESPACE;
                        }
                        '\n' => {
                            display_ch = '⏎';
                            fg = WHITESPACE;
                        }
                        '\r' => {
                            display_ch = '↤';
                            fg = TXT;
                            bg = CHAR_WARN;
                        }
                        '\t' => {
                            display_ch = '↦';
                            fg = TXT;
                            bg = CHAR_WARN;
                        }
                        _ => {}
                    }

                    let width = match ch {
                        ' ' | '\n' | '\r' => 1,
                        '\t' => TAB_WIDTH - (x % TAB_WIDTH),
                        ch => ch.width().unwrap_or(0),
                    };
                    if width == 0 {
                        continue;
                    }

                    // If any part of the character is visible, render that.
                    if x + width >= self.scroll_x && x < self.scroll_x + self.buff_w {
                        // Layer 2: Selection.
                        for selection in selections {
                            if selection.contains(Cursor::new(idx, doc_y)) {
                                bg = SEL;
                                break;
                            }
                        }

                        let display_y = self.y_off + y;

                        if x >= self.scroll_x {
                            let display_x = self.x_off + self.gutter_w + x - self.scroll_x;
                            display.update(Cell::new(display_ch, fg, bg), display_x, display_y);
                        }

                        // Layer 3: Expand tabs.
                        if ch == '\t' {
                            // Write as many spaces as needed after the tab character.
                            for n in 1..=width {
                                if x + n < self.scroll_x || x + n >= self.scroll_x + self.buff_w {
                                    continue;
                                }

                                let display_x = self.x_off + self.gutter_w + x + n - self.scroll_x;
                                display.update(Cell::new(' ', fg, bg), display_x, display_y);
                            }
                        } else {
                            // Mark all following cells of wide characters as taken.
                            for n in 1..width {
                                if x + n < self.scroll_x || x + n >= self.scroll_x + self.buff_w {
                                    continue;
                                }

                                // Use unknown character if the initial character was outside the viewport to avoid
                                // ghosting.
                                let display_ch = if x >= self.scroll_x {
                                    PLACEHOLDER
                                } else {
                                    '\u{FFFD}'
                                };
                                let display_x = self.x_off + self.gutter_w + x + n - self.scroll_x;
                                display.update(Cell::new(display_ch, fg, bg), display_x, display_y);
                            }
                        }
                    }

                    x += width;
                }
            }

            // Clear the remaining line.
            let base_bg = if doc_y == doc.cur.y { HIGHLIGHT } else { BG };
            let start = self.gutter_w + x.saturating_sub(self.scroll_x);
            for x in start..self.w {
                display.update(Cell::new(' ', TXT, base_bg), self.x_off + x, self.y_off + y);
            }
        }
    }

    /// Renders a vt100 parser state to the `Display`.
    pub fn render_terminal(&self, display: &mut Display, parser: &Parser) {
        let screen = parser.screen();

        // Render cells from the terminal screen.
        for y in 0..self.h {
            for x in 0..self.buff_w {
                // The indices are bound by terminal dimensions.
                #[allow(clippy::cast_possible_truncation)]
                if let Some(cell) = screen.cell(y as u16, x as u16) {
                    let Some(ch) = cell.contents().chars().next() else {
                        continue;
                    };
                    let fg = vt100_color_to_rgb(cell.fgcolor(), true);
                    let bg = vt100_color_to_rgb(cell.bgcolor(), false);

                    display.update(
                        Cell::new(ch, Fg(fg), Bg(bg)),
                        self.x_off + self.gutter_w + x,
                        self.y_off + y,
                    );
                } else {
                    // Default background if the cell doesn't contain data.
                    display.update(
                        Cell::new(' ', TXT, BG),
                        self.x_off + self.gutter_w + x,
                        self.y_off + y,
                    );
                }
            }
        }

        if screen.hide_cursor() {
            display.set_cursor(Cursor::new(0, 0), CursorStyle::Hidden);
        } else {
            let (row, col) = screen.cursor_position();
            let (x, y) = (col as usize, row as usize);

            display.set_cursor(
                Cursor::new(self.x_off + self.gutter_w + x, self.y_off + y),
                CursorStyle::SteadyBlock,
            );
        }
    }

    /// Renders line numbers to the `Display`.
    pub fn render_gutter(&mut self, display: &mut Display, doc: &Document) {
        if !self.gutter {
            return;
        }

        // Update the nums width if the supplied buffer is not correct.
        // log10 + 1 for length + 4 for whitespace and separator.
        if self.gutter && doc.len().ilog10() as usize + 5 != self.gutter_w {
            self.resize(self.w, self.h, self.x_off, self.y_off, Some(doc.len()));
        }

        for y in 0..self.h {
            let doc_y = self.scroll_y + y;
            let mut x = self.x_off;

            // Set base background color and move to the start of the line.
            let (base_bg, base_fg) = if doc_y == doc.cur.y {
                (HIGHLIGHT, TXT)
            } else {
                (BG, REL_NUMS)
            };

            // Skip screen lines outside the text line bounds.
            if doc_y >= doc.len() {
                for ch in format!("{}┃ ", " ".repeat(self.gutter_w - 2)).chars() {
                    display.update(Cell::new(ch, base_fg, base_bg), x, self.y_off + y);
                    x += 1;
                }
                continue;
            }

            let padding = self.gutter_w - 3;
            for ch in format!("{:>padding$} ┃ ", doc_y + 1).chars() {
                display.update(Cell::new(ch, base_fg, base_bg), x, self.y_off + y);
                x += 1;
            }
        }
    }

    /// Renders a bar to the `Display`.
    pub fn render_bar(&self, line: &str, y: usize, display: &mut Display) {
        let start = self.scroll_x;
        let end = (start + self.w).min(line.chars().count());

        let start_idx = line
            .char_indices()
            .nth(start)
            .map_or(line.len(), |(idx, _)| idx);
        let end_idx = line
            .char_indices()
            .nth(end)
            .map_or(line.len(), |(idx, _)| idx);
        let cmd = &line[start_idx..end_idx];

        let mut x = 0;
        for ch in cmd.chars() {
            let width = ch.width().unwrap_or(0);
            if width == 0 {
                continue;
            }

            if x + width <= self.w {
                display.update(Cell::new(ch, TXT, INFO), self.x_off + x, self.y_off + y);

                // Mark all following cells of wide characters as taken.
                for n in 1..width {
                    display.update(
                        Cell::new(PLACEHOLDER, TXT, INFO),
                        self.x_off + x + n,
                        self.y_off + y,
                    );
                }
            }
            x += width;
        }

        // Clear the remaining line.
        while x < self.w {
            display.update(Cell::new(' ', TXT, INFO), self.x_off + x, self.y_off + y);
            x += 1;
        }
    }

    /// Renders the `Cursor` of a `Document` to the `Display`.
    pub fn render_cursor(&self, display: &mut Display, doc: &Document, style: CursorStyle) {
        let line = doc
            .line(doc.cur.y)
            .map(|l| l.to_string())
            .unwrap_or_default();
        let visual_x = text_width(&line, doc.cur.x);

        let x = visual_x.saturating_sub(self.scroll_x);
        let y = doc.cur.y.saturating_sub(self.scroll_y);

        assert!(x < self.buff_w && y < self.h);
        display.set_cursor(
            Cursor::new(self.x_off + self.gutter_w + x, self.y_off + y),
            style,
        );
    }

    /// Shifts the viewport to the left.
    pub fn shift_left(&mut self, doc: &Document, n: usize) {
        let line = doc
            .line(doc.cur.y)
            .map(|l| l.to_string())
            .unwrap_or_default();
        let x = text_width(&line, doc.cur.x);

        self.scroll_x = (self.scroll_x + n).min(x);
    }

    /// Shifts the viewport to the right.
    pub fn shift_right(&mut self, doc: &Document, n: usize) {
        let line = doc
            .line(doc.cur.y)
            .map(|l| l.to_string())
            .unwrap_or_default();
        let x = text_width(&line, doc.cur.x);

        let limit = (x + 1).saturating_sub(self.buff_w);
        self.scroll_x = self.scroll_x.saturating_sub(n).max(limit);
    }

    /// Shifts the viewport up.
    pub fn shift_up(&mut self, doc: &Document, n: usize) {
        self.scroll_y = (self.scroll_y + n).min(doc.cur.y);
    }

    /// Shifts the viewport up.
    pub fn shift_down(&mut self, doc: &Document, n: usize) {
        let limit = (doc.cur.y + 1).saturating_sub(self.h);
        self.scroll_y = self.scroll_y.saturating_sub(n).max(limit);
    }
}
