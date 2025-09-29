mod interact;
mod r#move;

use crate::{
    INFO_BUFF_IDX, TXT_BUFF_IDX,
    buffer::Buffer,
    document::Document,
    util::{CommandResult, CursorStyle},
    viewport::Viewport,
};
use std::{
    io::{BufWriter, Error, Stdout},
    path::PathBuf,
};
use termion::{event::Key, raw::RawTerminal};

pub struct FilesBuffer {
    doc: Document,
    view: Viewport,
    base: PathBuf,
    entries: Vec<PathBuf>,
}

impl FilesBuffer {
    pub fn new(w: usize, h: usize, base: PathBuf) -> Result<Self, Error> {
        let mut entries = Vec::new();
        let mut contents = Vec::new();
        FilesBuffer::load_dir(&base, &mut entries, &mut contents)?;

        Ok(FilesBuffer {
            doc: Document::new(Some(contents), 0, 0),
            view: Viewport::new(w, h, 0, h / 2),
            base,
            entries,
        })
    }

    fn info_line(&self) -> String {
        use std::fmt::Write;

        let mut info_line = String::new();

        // No plus 1 since the first entry is always ".." and not really a directory entry.
        let curr = self.doc.cursor.y;
        let curr_type = match curr {
            0 => "Parent Dir",
            idx if self.entries[idx - 1].is_symlink() => "Symlink",
            idx if self.entries[idx - 1].is_dir() => "Dir",
            _ => "File",
        };
        let entries = self.entries.len();
        let entries_label = if entries == 1 { "Entry" } else { "Entries" };

        write!(
            &mut info_line,
            "[Files] [{curr_type}] [{curr}/{entries} {entries_label}]",
        )
        .unwrap();

        info_line
    }
}

impl Buffer for FilesBuffer {
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        self.view.render(
            stdout,
            &self.doc,
            self.info_line(),
            None,
            CursorStyle::SteadyBlock,
        )
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.w == w && self.view.h == h {
            return;
        }

        self.view.resize(w, h, self.view.cursor.x.min(w), h / 2);
    }

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
            Key::Char('\n') => {
                return self
                    .select_item()
                    .or_else(|err| {
                        Ok::<CommandResult, Error>(CommandResult::SetAndChangeBuffer(
                            INFO_BUFF_IDX,
                            vec![err.to_string()],
                        ))
                    })
                    .unwrap();
            }
            Key::Char('t') => return CommandResult::ChangeBuffer(TXT_BUFF_IDX),
            Key::Char('?') => return CommandResult::ChangeBuffer(INFO_BUFF_IDX),
            _ => {}
        }

        CommandResult::Ok
    }

    fn set_contents(&mut self, _: &[String]) {
        unreachable!("Contents of FileTreeBuffer cannot be set")
    }
}
