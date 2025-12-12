use crate::{
    cursor::{Cursor, CursorStyle},
    display::{Cell, Display},
    document::Document,
    message::{Message, MessageKind},
    selection::Selection,
    shell_command::util::vt100_color_to_rgb,
};
use termion::color::{self, Bg, Fg};
use vt100::Parser;

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

    /// Recalculates the visible viewport area.
    pub const fn recalculate_viewport(&mut self, cursor: &Cursor) {
        // Horizontal Scrolling
        if cursor.x < self.scroll_x {
            self.scroll_x = cursor.x;
        } else if cursor.x >= self.scroll_x + self.buff_w {
            self.scroll_x = cursor.x.saturating_sub(self.buff_w) + 1;
        }

        // Vertical Scrolling
        if cursor.y < self.scroll_y {
            self.scroll_y = cursor.y;
        } else if cursor.y >= self.scroll_y + self.h {
            self.scroll_y = cursor.y.saturating_sub(self.h) + 1;
        }
    }

    /// Sets the gutter width.
    pub const fn set_gutter_width(&mut self, n: usize) {
        self.gutter_w = n + 4;
        self.buff_w = self.w - n - 4;
    }

    /// Renders a message overlay to the `Display`. Should be called after `render_document` because it will get
    /// overwritten otherwise.
    pub fn render_message(&self, display: &mut Display, message: &Message) {
        let mut chars = message.text.chars();

        // Skip lines that are "scrolled off" the screen.
        for _ in 0..message.scroll {
            for _ in 0..self.w {
                match chars.next() {
                    Some('\n') => break,
                    Some(_) => {}
                    None => unreachable!(),
                }
            }
        }

        // Skip the "scrolled off" lines and only show at most 1/3rd of the height of error.
        for y in 0..(message.lines - message.scroll).min(self.h / 3) {
            let mut newline = false;
            for x in 0..self.w {
                if newline {
                    // Fill the remaining line.
                    display.update(
                        Cell::new(' ', ERROR_TXT, INFO),
                        self.x_off + x,
                        self.y_off + y,
                    );
                } else {
                    let mut display_ch = chars.next().unwrap_or(' ');

                    if display_ch == '\n' {
                        newline = true;
                        display.update(
                            Cell::new(' ', ERROR_TXT, INFO),
                            self.x_off + x,
                            self.y_off + y,
                        );
                    } else {
                        let mut fg = match message.kind {
                            MessageKind::Info => INFO_TXT,
                            MessageKind::Error => ERROR_TXT,
                        };
                        let mut bg = INFO;

                        // Layer 1: Character replacement.
                        if display_ch == '\r' {
                            display_ch = '↤';
                            fg = TXT;
                            bg = CHAR_WARN;
                        } else if display_ch == '\t' {
                            display_ch = '↦';
                            fg = TXT;
                            bg = CHAR_WARN;
                        }

                        display.update(
                            Cell::new(display_ch, fg, bg),
                            self.x_off + x,
                            self.y_off + y,
                        );
                    }
                }
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

            let Some(line) = doc.line(doc_y) else {
                break;
            };

            for (x, ch) in line.chars().enumerate() {
                if x >= self.scroll_x && x < self.scroll_x + self.buff_w {
                    let mut display_ch = ch;
                    let mut fg = TXT;
                    let mut bg = if doc_y == doc.cur.y { HIGHLIGHT } else { BG };

                    // Layer 1: Selection.
                    for selection in selections {
                        if selection.contains(Cursor::new(x, doc_y)) {
                            bg = SEL;
                            break;
                        }
                    }

                    // Layer 2: Character replacement.
                    if display_ch == ' ' {
                        display_ch = '·';
                        fg = WHITESPACE;
                    } else if display_ch == '\n' {
                        display_ch = '⏎';
                        fg = WHITESPACE;
                    } else if display_ch == '\r' {
                        display_ch = '↤';
                        fg = TXT;
                        bg = CHAR_WARN;
                    } else if display_ch == '\t' {
                        display_ch = '↦';
                        fg = TXT;
                        bg = CHAR_WARN;
                    }

                    let x = self.x_off + self.gutter_w + x - self.scroll_x;
                    let y = self.y_off + y;
                    display.update(Cell::new(display_ch, fg, bg), x, y);
                }
            }
        }

        // Render trailing whitespace to override previous screen content. The previous loop only renders the current
        // content without regard of removing existing content, which is why this second render pass is necessary.
        for y in 0..self.h {
            let doc_y = self.scroll_y + y;

            // Set base background color depending on if its the cursors line.
            let base_bg = if doc_y == doc.cur.y { HIGHLIGHT } else { BG };

            // Skip screen lines outside the text line bounds.
            if doc_y >= doc.len() {
                for x in self.gutter_w..self.w {
                    display.update(Cell::new(' ', TXT, base_bg), self.x_off + x, self.y_off + y);
                }
                continue;
            }

            let len = doc.line_count(doc_y).unwrap();
            // Calculate the end of the line contents.
            let x = self.gutter_w + len.saturating_sub(self.scroll_x);

            // Stretch current line to end to show highlight properly.
            for x in x..self.w {
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
                    let ch = cell.contents().chars().next().unwrap_or(' ');
                    let fg = vt100_color_to_rgb(cell.fgcolor(), true);
                    let bg = vt100_color_to_rgb(cell.bgcolor(), false);

                    display.update(
                        Cell::new(ch, Fg(fg), Bg(bg)),
                        self.gutter_w + x + self.x_off,
                        y + self.y_off,
                    );
                } else {
                    // Default background if the cell doesn't contain data.
                    display.update(
                        Cell::new(' ', TXT, BG),
                        self.gutter_w + x + self.x_off,
                        y + self.y_off,
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
                Cursor::new(self.gutter_w + x + self.x_off, y + self.y_off),
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
        let padding = self.w.saturating_sub(cmd.chars().count());

        for (x, ch) in format!("{cmd}{}", " ".repeat(padding)).chars().enumerate() {
            display.update(Cell::new(ch, TXT, INFO), x + self.x_off, y + self.y_off);
        }
    }

    /// Renders a `Cursor` to the `Display`.
    pub const fn render_cursor(
        &self,
        display: &mut Display,
        cur: &Cursor,
        cursor_style: CursorStyle,
    ) {
        display.set_cursor(
            Cursor::new(self.gutter_w + cur.x + self.x_off, cur.y + self.y_off),
            cursor_style,
        );
    }
}
