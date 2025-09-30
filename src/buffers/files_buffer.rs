mod interact;
mod r#move;

use crate::{
    INFO_BUFF_IDX, TXT_BUFF_IDX,
    buffer::Buffer,
    document::Document,
    state_machine::{CommandMap, StateMachine},
    util::{CommandResult, CursorStyle},
    viewport::Viewport,
};
use std::{
    io::{BufWriter, Error, Stdout},
    path::PathBuf,
    time::Duration,
};
use termion::{event::Key, raw::RawTerminal};

#[derive(Clone)]
enum Action {
    Left,
    Down,
    Up,
    Right,
    NextWord,
    PrevWord,
    JumpToBeginningOfLine,
    JumpToEndOfLine,
    JumpToMatchingOpposite,
    JumpToBeginningOfFile,
    JumpToEndOfFile,
    Refresh,
    SelectItem,
    ChangeToTextBuffer,
    ChangeToInfoBuffer,
}

pub struct FilesBuffer {
    doc: Document,
    view: Viewport,
    base: PathBuf,
    entries: Vec<PathBuf>,
    input_state_machine: StateMachine<Action>,
}

impl FilesBuffer {
    pub fn new(w: usize, h: usize, base: PathBuf) -> Result<Self, Error> {
        let mut entries = Vec::new();
        let mut contents = Vec::new();
        FilesBuffer::load_dir(&base, &mut entries, &mut contents)?;

        let command_map = CommandMap::new()
            .simple(Key::Char('h'), Action::Left)
            .simple(Key::Char('j'), Action::Down)
            .simple(Key::Char('k'), Action::Up)
            .simple(Key::Char('l'), Action::Right)
            .simple(Key::Char('w'), Action::NextWord)
            .simple(Key::Char('b'), Action::PrevWord)
            .simple(Key::Char('<'), Action::JumpToBeginningOfLine)
            .simple(Key::Char('>'), Action::JumpToEndOfLine)
            .simple(Key::Char('.'), Action::JumpToMatchingOpposite)
            .simple(Key::Char('g'), Action::JumpToEndOfFile)
            .simple(Key::Char('G'), Action::JumpToBeginningOfFile)
            .simple(Key::Char('r'), Action::Refresh)
            .simple(Key::Char('\n'), Action::SelectItem)
            .simple(Key::Char('t'), Action::ChangeToTextBuffer)
            .simple(Key::Char('?'), Action::ChangeToInfoBuffer);
        let input_state_machine = StateMachine::new(command_map, Duration::from_secs(1));

        Ok(FilesBuffer {
            doc: Document::new(Some(contents), 0, 0),
            view: Viewport::new(w, h, 0, h / 2),
            base,
            entries,
            input_state_machine,
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
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};

        match self.input_state_machine.tick(key.into()) {
            A(Action::Left) => self.left(1),
            A(Action::Down) => self.down(1),
            A(Action::Up) => self.up(1),
            A(Action::Right) => self.right(1),
            A(Action::NextWord) => self.next_word(),
            A(Action::PrevWord) => self.prev_word(),
            A(Action::JumpToBeginningOfLine) => self.jump_to_beginning_of_line(),
            A(Action::JumpToEndOfLine) => self.jump_to_end_of_line(),
            A(Action::JumpToMatchingOpposite) => self.jump_to_matching_opposite(),
            A(Action::JumpToEndOfFile) => self.jump_to_end_of_file(),
            A(Action::JumpToBeginningOfFile) => self.jump_to_beginning_of_file(),
            A(Action::Refresh) => {
                if let Err(err) =
                    FilesBuffer::load_dir(&self.base, &mut self.entries, &mut self.doc.lines)
                {
                    return CommandResult::SetAndChangeBuffer(INFO_BUFF_IDX, vec![err.to_string()]);
                }
            }
            A(Action::SelectItem) => {
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
            A(Action::ChangeToTextBuffer) => return CommandResult::ChangeBuffer(TXT_BUFF_IDX),
            A(Action::ChangeToInfoBuffer) => return CommandResult::ChangeBuffer(INFO_BUFF_IDX),
            Incomplete | Invalid => {}
        }

        CommandResult::Ok
    }

    fn set_contents(&mut self, _: &[String]) {
        unreachable!("Contents of FilesBuffer cannot be set")
    }

    fn can_quit(&self) -> Result<(), Vec<String>> {
        Ok(())
    }
}
