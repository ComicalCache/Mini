mod interact;

use crate::{
    INFO_BUFF_IDX, TXT_BUFF_IDX,
    buffer::Buffer,
    cursor_move as cm,
    document::Document,
    state_machine::{CommandMap, StateMachine},
    util::{CommandResult, CursorStyle},
    viewport::Viewport,
};
use std::{
    borrow::Cow,
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

        let input_state_machine = {
            #[allow(clippy::enum_glob_use)]
            use Action::*;

            let command_map = CommandMap::new()
                .simple(Key::Char('h'), Left)
                .simple(Key::Char('j'), Down)
                .simple(Key::Char('k'), Up)
                .simple(Key::Char('l'), Right)
                .simple(Key::Char('w'), NextWord)
                .simple(Key::Char('b'), PrevWord)
                .simple(Key::Char('<'), JumpToBeginningOfLine)
                .simple(Key::Char('>'), JumpToEndOfLine)
                .simple(Key::Char('.'), JumpToMatchingOpposite)
                .simple(Key::Char('g'), JumpToEndOfFile)
                .simple(Key::Char('G'), JumpToBeginningOfFile)
                .simple(Key::Char('r'), Refresh)
                .simple(Key::Char('\n'), SelectItem)
                .simple(Key::Char('t'), ChangeToTextBuffer)
                .simple(Key::Char('?'), ChangeToInfoBuffer);
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        Ok(FilesBuffer {
            doc: Document::new(0, 0, Some(contents)),
            view: Viewport::new(w, h, 0, h / 2),
            base,
            entries,
            input_state_machine,
        })
    }

    fn info_line(&mut self) {
        use std::fmt::Write;

        self.view.info_line.clear();

        // No plus 1 since the first entry is always ".." and not really a directory entry.
        let curr = self.doc.cur.y;
        let curr_type = match curr {
            0 => "Parent Dir",
            idx if self.entries[idx - 1].is_symlink() => "Symlink",
            idx if self.entries[idx - 1].is_dir() => "Dir",
            _ => "File",
        };
        let entries = self.entries.len();
        let entries_label = if entries == 1 { "Entry" } else { "Entries" };

        write!(
            &mut self.view.info_line,
            "[Files] [{curr_type}] [{curr}/{entries} {entries_label}]",
        )
        .unwrap();
    }
}

impl Buffer for FilesBuffer {
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        self.info_line();
        self.view.cmd = None;
        self.view
            .render(stdout, &self.doc, CursorStyle::SteadyBlock)
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.w == w && self.view.h == h {
            return;
        }

        self.view.resize(w, h, self.view.cur.x.min(w), h / 2);
    }

    fn tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use Action::*;

        match self.input_state_machine.tick(key.into()) {
            A(Left) => cm::left(&mut self.doc, &mut self.view, 1),
            A(Down) => cm::down(&mut self.doc, &mut self.view, 1),
            A(Up) => cm::up(&mut self.doc, &mut self.view, 1),
            A(Right) => cm::right(&mut self.doc, &mut self.view, 1),
            A(NextWord) => cm::next_word(&mut self.doc, &mut self.view),
            A(PrevWord) => cm::prev_word(&mut self.doc, &mut self.view),
            A(JumpToBeginningOfLine) => {
                cm::jump_to_beginning_of_line(&mut self.doc, &mut self.view);
            }
            A(JumpToEndOfLine) => cm::jump_to_end_of_line(&mut self.doc, &mut self.view),
            A(JumpToMatchingOpposite) => {
                cm::jump_to_matching_opposite(&mut self.doc, &mut self.view);
            }
            A(JumpToEndOfFile) => cm::jump_to_end_of_file(&mut self.doc, &mut self.view),
            A(JumpToBeginningOfFile) => {
                cm::jump_to_beginning_of_file(&mut self.doc, &mut self.view);
            }
            A(Refresh) => {
                if let Err(err) =
                    FilesBuffer::load_dir(&self.base, &mut self.entries, &mut self.doc.buff)
                {
                    return CommandResult::SetAndChangeBuffer(
                        INFO_BUFF_IDX,
                        vec![Cow::from(err.to_string())],
                        None,
                    );
                }
            }
            A(SelectItem) => {
                return self
                    .select_item()
                    .or_else(|err| {
                        Ok::<CommandResult, Error>(CommandResult::SetAndChangeBuffer(
                            INFO_BUFF_IDX,
                            vec![Cow::from(err.to_string())],
                            None,
                        ))
                    })
                    .unwrap();
            }
            A(ChangeToTextBuffer) => return CommandResult::ChangeBuffer(TXT_BUFF_IDX),
            A(ChangeToInfoBuffer) => return CommandResult::ChangeBuffer(INFO_BUFF_IDX),
            Incomplete | Invalid => {}
        }

        CommandResult::Ok
    }

    fn set_contents(&mut self, _: &[Cow<'static, str>], path: Option<PathBuf>) {
        if let Some(path) = path {
            self.base = path;
        }
    }

    fn can_quit(&self) -> Result<(), Vec<Cow<'static, str>>> {
        Ok(())
    }
}
