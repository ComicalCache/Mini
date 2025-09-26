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
pub enum TextBufferMode {
    View,
    Write,
    Command,
}

pub struct TextBuffer {
    doc: Document,
    view: Viewport,
    file: Option<File>,
    mode: TextBufferMode,
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
            mode: TextBufferMode::View,
        })
    }

    fn info_line(&self) -> String {
        use std::fmt::Write;

        let mut info_line = String::new();

        let mode = match self.mode {
            TextBufferMode::View => "V",
            TextBufferMode::Write => "W",
            TextBufferMode::Command => "C",
        };
        // Plus 1 since text coordinates are 0 indexed
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
            TextBufferMode::View => CursorStyle::BlinkingBlock,
            TextBufferMode::Write | TextBufferMode::Command => CursorStyle::BlinkingBar,
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

        match key {
            Key::Char('q') => CommandResult::Quit,
            Key::Char('h') => {
                self.doc.cursor.left(1);
                self.view.cursor.left(1);

                CommandResult::Ok
            }
            Key::Char('j') => {
                let bound = self.doc.lines.len().saturating_sub(1);
                self.doc.cursor.down(1, bound);

                // When moving down, handle case that new line contains less text than previous
                let line_bound = self.doc.lines[self.doc.cursor.y].chars().count();
                if self.doc.cursor.x >= line_bound {
                    let diff = self.doc.cursor.x - line_bound;
                    self.doc.cursor.left(diff);
                    self.view.cursor.left(diff);
                }

                CommandResult::Ok
            }
            Key::Char('k') => {
                self.doc.cursor.up(1);

                // When moving up, handle case that new line contains less text than previous
                let line_bound = self.doc.lines[self.doc.cursor.y].chars().count();
                if self.doc.cursor.x >= line_bound {
                    let diff = self.doc.cursor.x - line_bound;
                    self.doc.cursor.left(diff);
                    self.view.cursor.left(diff);
                }

                CommandResult::Ok
            }
            Key::Char('l') => {
                let line_bound = self.doc.lines[self.doc.cursor.y].chars().count();
                self.doc.cursor.right(1, line_bound);
                self.view.cursor.right(1, line_bound.min(self.view.w - 1));

                CommandResult::Ok
            }
            _ => CommandResult::Ok,
        }
    }
}
