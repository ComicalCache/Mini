use std::io::{BufWriter, Error, Stdout, Write};
use termion::{
    color::{self, Bg, Fg, Reset},
    cursor::{Goto, Hide, Show, SteadyBar, SteadyBlock},
    raw::RawTerminal,
};

use crate::cursor::{Cursor, CursorStyle};

/// Use the placeholder U+FFFF value to indicate a cell is taken by wide characters.
pub const PLACEHOLDER: char = '\u{FFFF}';

/// Reset text color.
const NO_TXT: Fg<Reset> = Fg(Reset);
/// Reset background color.
const NO_BG: Bg<Reset> = Bg(Reset);

/// A display buffer.
pub struct Display {
    /// The cells of the display.
    buff: Vec<Vec<Cell>>,
    /// List of cell coordinates that need to be redrawn.
    redraw: Vec<(usize, usize)>,
    /// The cursor on the display.
    cursor: (Cursor, CursorStyle),

    /// The width of the display.
    w: usize,
    /// The height of the display.
    h: usize,

    /// Issuing a full redraw on resize.
    full_redraw: bool,
}

impl Display {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            buff: vec![vec![Cell::default(); w]; h],
            redraw: Vec::new(),
            cursor: (Cursor::new(0, 0), CursorStyle::SteadyBlock),
            w,
            h,
            full_redraw: false,
        }
    }

    /// Resizes the display.
    pub fn resize(&mut self, w: usize, h: usize) {
        let mut redraw = false;
        // Resize line width first to avoid more work.
        if self.w != w {
            self.w = w;
            redraw = true;

            for line in &mut self.buff {
                line.resize(w, Cell::default());
            }
        }

        // Resize height second.
        if self.h != h {
            self.h = h;
            redraw = true;

            self.buff.resize(h, vec![Cell::default(); w]);
        }

        // Redraw everything on resize.
        self.full_redraw = redraw;
    }

    /// Updates a cell in the display.
    pub fn update(&mut self, cell: Cell, x: usize, y: usize) {
        if self.buff[y][x] != cell {
            self.buff[y][x] = cell;
            self.redraw.push((x, y));
        }
    }

    /// Sets the cursor of the display.
    pub const fn set_cursor(&mut self, cursor: Cursor, style: CursorStyle) {
        self.cursor = (cursor, style);
    }

    /// Draws the display to the terminal.
    pub fn draw(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        // Hide the cursor to avoid it flickering over the screen.
        write!(stdout, "{Hide}")?;

        if self.full_redraw {
            write!(stdout, "{NO_TXT}{NO_BG}")?;
            write!(stdout, "{}", termion::clear::All)?;

            // Store last used colors to not write the color for ever character.
            let mut last_fg: Option<Fg<color::Rgb>> = None;
            let mut last_bg: Option<Bg<color::Rgb>> = None;
            for y in 0..self.h {
                for x in 0..self.w {
                    self.draw_cell(x, y, &mut last_fg, &mut last_bg, stdout)?;
                }
            }
            self.full_redraw = false;
        } else if !self.redraw.is_empty() {
            // Store last used colors to not write the color for ever character.
            let mut last_fg: Option<Fg<color::Rgb>> = None;
            let mut last_bg: Option<Bg<color::Rgb>> = None;
            for (x, y) in &self.redraw {
                self.draw_cell(*x, *y, &mut last_fg, &mut last_bg, stdout)?;
            }
            self.redraw.clear();
        }

        // Always draw the cursor.
        // The cursor is bound by the terminal dimensions.
        #[allow(clippy::cast_possible_truncation)]
        let cur = Goto(self.cursor.0.x as u16 + 1, self.cursor.0.y as u16 + 1);
        match self.cursor.1 {
            CursorStyle::Hidden => {}
            CursorStyle::SteadyBar => write!(stdout, "{cur}{SteadyBar}{Show}")?,
            CursorStyle::SteadyBlock => write!(stdout, "{cur}{SteadyBlock}{Show}")?,
        }

        write!(stdout, "{NO_TXT}{NO_BG}")?;
        stdout.flush()
    }

    fn draw_cell(
        &self,
        x: usize,
        y: usize,
        last_fg: &mut Option<Fg<color::Rgb>>,
        last_bg: &mut Option<Bg<color::Rgb>>,
        stdout: &mut BufWriter<RawTerminal<Stdout>>,
    ) -> Result<(), Error> {
        let Cell { ch, fg, bg, .. } = self.buff[y][x];

        if ch == PLACEHOLDER {
            return Ok(());
        }

        // The indices are bound by terminal dimensions.
        #[allow(clippy::cast_possible_truncation)]
        write!(stdout, "{}", Goto(x as u16 + 1, y as u16 + 1))?;

        // Write colors if necessary.
        match last_fg {
            Some(last_fg) if last_fg.0 == fg.0 => {}
            _ => {
                write!(stdout, "{fg}")?;
                *last_fg = Some(fg);
            }
        }
        match last_bg {
            Some(last_bg) if last_bg.0 == bg.0 => {}
            _ => {
                write!(stdout, "{bg}")?;
                *last_bg = Some(bg);
            }
        }

        write!(stdout, "{ch}")
    }
}

/// A cell of the display.
#[derive(Clone)]
pub struct Cell {
    /// The character at that cell.
    pub ch: char,
    /// The foreground color at that cell.
    pub fg: Fg<color::Rgb>,
    /// The background color at that cell.
    pub bg: Bg<color::Rgb>,
}

impl Cell {
    pub const fn new(ch: char, fg: Fg<color::Rgb>, bg: Bg<color::Rgb>) -> Self {
        Self { ch, fg, bg }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: '\0',
            fg: Fg(color::Rgb(0, 0, 0)),
            bg: Bg(color::Rgb(0, 0, 0)),
        }
    }
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.ch == other.ch && self.fg.0 == other.fg.0 && self.bg.0 == other.bg.0
    }
}
