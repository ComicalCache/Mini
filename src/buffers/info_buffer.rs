mod r#move;

use crate::{
    TXT_BUFF_IDX,
    document::Document,
    traits::{Buffer, Contents, Render, Tick},
    util::{CommandResult, CursorStyle},
    viewport::Viewport,
};
use std::io::{BufWriter, Error, Stdout};
use termion::{event::Key, raw::RawTerminal};

pub struct InfoBuffer {
    doc: Document,
    view: Viewport,
}

impl InfoBuffer {
    pub fn new(w: usize, h: usize) -> Self {
        InfoBuffer {
            doc: Document::new(None, 0, 0),
            view: Viewport::new(w, h, 0, h / 2),
        }
    }

    fn info_line(&self) -> String {
        use std::fmt::Write;

        let mut info_line = String::new();

        // Plus 1 since text coordinates are 0 indexed.
        let line = self.doc.cursor.y + 1;
        let col = self.doc.cursor.x + 1;
        let total = self.doc.lines.len();
        let percentage = 100 * line / total;
        let size: usize = self.doc.lines.iter().map(String::len).sum();

        write!(
            &mut info_line,
            "[Info] [{line}:{col}/{total} {percentage}%] [{size}B]",
        )
        .unwrap();

        let edited = if self.doc.edited { '*' } else { ' ' };
        write!(&mut info_line, " {edited}").unwrap();

        info_line
    }
}

impl Buffer for InfoBuffer {}

impl Render for InfoBuffer {
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        self.view.render(
            stdout,
            &self.doc,
            self.info_line(),
            None,
            CursorStyle::BlinkingBlock,
        )
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.w == w && self.view.h == h {
            return;
        }

        self.view.resize(w, h, self.view.cursor.x.min(w), h / 2);
    }
}

impl Tick for InfoBuffer {
    fn tick(&mut self, key: Option<Key>) -> CommandResult {
        let Some(key) = key else {
            return CommandResult::Ok;
        };

        match key {
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
            Key::Char('?') => return CommandResult::ChangeBuffer(TXT_BUFF_IDX),
            _ => {}
        }

        CommandResult::Ok
    }
}

impl Contents for InfoBuffer {
    fn set_contents(&mut self, contents: &[String]) {
        self.doc.set_contents(contents, 0, 0);
    }
}
