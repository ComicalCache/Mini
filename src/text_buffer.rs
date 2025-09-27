mod edit;
mod r#move;

use crate::{
    document::Document,
    traits::{Buffer, Render, Tick},
    util::{CommandResult, CursorStyle, read_file_to_lines},
    viewport::Viewport,
};
use std::{
    fs::File,
    io::{BufWriter, Error, Stdout},
};
use termion::{event::Key, raw::RawTerminal};

#[derive(Clone, Copy)]
enum Mode {
    View,
    Write,
    Command,
}

pub struct TextBuffer {
    doc: Document,
    view: Viewport,
    file: Option<File>,
    mode: Mode,
}

impl TextBuffer {
    pub fn new(w: usize, h: usize, mut file: Option<File>) -> Result<Self, Error> {
        let content = if let Some(file) = file.as_mut() {
            Some(read_file_to_lines(file)?)
        } else {
            None
        };

        Ok(TextBuffer {
            doc: Document::new(content),
            view: Viewport::new(w, h, 0, h / 2),
            file,
            mode: Mode::View,
        })
    }

    fn info_line(&self) -> String {
        use std::fmt::Write;

        let mut info_line = String::new();

        let mode = match self.mode {
            Mode::View => "V",
            Mode::Write => "W",
            Mode::Command => "C",
        };
        // Plus 1 since text coordinates are 0 indexed.
        let line = self.doc.cursor.y + 1;
        let col = self.doc.cursor.x + 1;
        let total = self.doc.lines.len();
        let percentage = 100 * line / total;
        let size: usize = self.doc.lines.iter().map(String::len).sum();

        write!(
            &mut info_line,
            "[Text] [{mode}] [{line}:{col}/{total} {percentage}%] [{size}B]",
        )
        .unwrap();
        // if let Some(pos) = self.select {
        //     // Plus 1 since text coordinates are 0 indexed
        //     let line = pos.y + 1;
        //     let col = pos.x + 1;
        //     write!(
        //         &mut self.screen_buff[screen_idx],
        //         " [Selected {line}:{col}]"
        //     )?;
        // }

        let edited = if self.doc.edited { '*' } else { ' ' };
        write!(&mut info_line, " {edited}").unwrap();

        info_line
    }
}

impl Buffer for TextBuffer {}

impl Render for TextBuffer {
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        let cursor_style = match self.mode {
            Mode::View => CursorStyle::BlinkingBlock,
            Mode::Write | Mode::Command => CursorStyle::BlinkingBar,
        };

        // TODO: update for command line mode
        self.view
            .render(stdout, &self.doc, &self.info_line(), None, cursor_style)
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.w == w && self.view.h == h {
            return;
        }

        self.view.resize(w, h, self.view.cursor.x.min(w), h / 2);
    }
}

impl Tick for TextBuffer {
    fn tick(&mut self, key: Option<Key>) -> CommandResult {
        let Some(key) = key else {
            return CommandResult::Ok;
        };

        match self.mode {
            Mode::View => match key {
                Key::Char('q') => return CommandResult::Quit,
                Key::Char('i') => self.mode = Mode::Write,
                Key::Char('a') => {
                    self.right(1);
                    self.mode = Mode::Write;
                }
                Key::Char('o') => {
                    self.insert_move_new_line_bellow();
                    self.mode = Mode::Write;
                }
                Key::Char('O') => {
                    self.insert_move_new_line_above();
                    self.mode = Mode::Write;
                }
                Key::Char('h') => self.left(1),
                Key::Char('j') => self.down(1),
                Key::Char('k') => self.up(1),
                Key::Char('l') => self.right(1),
                Key::Char('w') => self.next_word(),
                Key::Char('b') => self.prev_word(),
                Key::Char('<') => self.jump_to_beginning_of_line(),
                Key::Char('>') => self.jump_to_end_of_line(),
                Key::Char('.') => self.jump_to_matching_opposite(),
                Key::Char('g') => self.jump_to_end_of_file(),
                Key::Char('G') => self.jump_to_beginning_of_file(),
                _ => {}
            },
            Mode::Write => match key {
                Key::Esc => self.mode = Mode::View,
                Key::Left => self.left(1),
                Key::Down => self.down(1),
                Key::Up => self.up(1),
                Key::Right => self.right(1),
                Key::Char('\n') => self.write_new_line_char(),
                Key::Char('\t') => self.write_tab(),
                Key::Char(ch) => self.write_char(ch),
                Key::Backspace => self.delete_char(),
                _ => {}
            },
            Mode::Command => {}
        }

        CommandResult::Ok
    }
}
