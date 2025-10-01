mod interact;

use crate::{
    INFO_BUFF_IDX, TXT_BUFF_IDX,
    buffer::{Buffer, yank},
    cursor::{self, Cursor},
    document::Document,
    state_machine::{ChainResult, CommandMap, StateMachine},
    util::{CommandResult, CursorStyle},
    viewport::Viewport,
};
use arboard::Clipboard;
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
    SelectMode,
    ExitSelectMode,
    YankSelection,
    YankLine,
    YankLeft,
    YankRight,
    YankNextWord,
    YankPrevWord,
    YankToBeginningOfLine,
    YankToEndOfLine,
    YankToMatchingOpposite,
    YankToBeginningOfFile,
    YankToEndOfFile,
    ChangeToTextBuffer,
    ChangeToInfoBuffer,
    Repeat(char),
}

pub struct FilesBuffer {
    doc: Document,
    view: Viewport,
    base: PathBuf,
    entries: Vec<PathBuf>,

    selection: Option<Cursor>,
    motion_repeat: String,
    clipboard: Clipboard,
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
                .simple(Key::Char('v'), SelectMode)
                .simple(Key::Esc, ExitSelectMode)
                .operator(Key::Char('y'), |key| match key {
                    Key::Char('v') => Some(ChainResult::Action(YankSelection)),
                    Key::Char('y') => Some(ChainResult::Action(YankLine)),
                    Key::Char('h') => Some(ChainResult::Action(YankLeft)),
                    Key::Char('l') => Some(ChainResult::Action(YankRight)),
                    Key::Char('w') => Some(ChainResult::Action(YankNextWord)),
                    Key::Char('b') => Some(ChainResult::Action(YankPrevWord)),
                    Key::Char('<') => Some(ChainResult::Action(YankToBeginningOfLine)),
                    Key::Char('>') => Some(ChainResult::Action(YankToEndOfLine)),
                    Key::Char('.') => Some(ChainResult::Action(YankToMatchingOpposite)),
                    Key::Char('g') => Some(ChainResult::Action(YankToEndOfFile)),
                    Key::Char('G') => Some(ChainResult::Action(YankToBeginningOfFile)),
                    _ => None,
                })
                .simple(Key::Char('t'), ChangeToTextBuffer)
                .simple(Key::Char('?'), ChangeToInfoBuffer)
                .simple(Key::Char('0'), Repeat('0'))
                .simple(Key::Char('1'), Repeat('1'))
                .simple(Key::Char('2'), Repeat('2'))
                .simple(Key::Char('3'), Repeat('3'))
                .simple(Key::Char('4'), Repeat('4'))
                .simple(Key::Char('5'), Repeat('5'))
                .simple(Key::Char('6'), Repeat('6'))
                .simple(Key::Char('7'), Repeat('7'))
                .simple(Key::Char('8'), Repeat('8'))
                .simple(Key::Char('9'), Repeat('9'));
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        Ok(FilesBuffer {
            doc: Document::new(0, 0, Some(contents)),
            view: Viewport::new(w, h, 0, h / 2),
            base,
            entries,
            selection: None,
            motion_repeat: String::new(),
            clipboard: Clipboard::new().map_err(Error::other)?,
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

        if let Some(pos) = self.selection {
            // Plus 1 since text coordinates are 0 indexed.
            let line = pos.y + 1;
            let col = pos.x + 1;
            write!(&mut self.view.info_line, " [Selected {line}:{col}]").unwrap();
        }
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
            A(Left) => cursor::left(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(Down) => cursor::down(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(Up) => cursor::up(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(Right) => cursor::right(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(NextWord) => cursor::next_word(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(PrevWord) => cursor::prev_word(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(JumpToBeginningOfLine) => {
                cursor::jump_to_beginning_of_line(&mut self.doc, &mut self.view);
            }
            A(JumpToEndOfLine) => cursor::jump_to_end_of_line(&mut self.doc, &mut self.view),
            A(JumpToMatchingOpposite) => {
                cursor::jump_to_matching_opposite(&mut self.doc, &mut self.view);
            }
            A(JumpToEndOfFile) => cursor::jump_to_end_of_file(&mut self.doc, &mut self.view),
            A(JumpToBeginningOfFile) => {
                cursor::jump_to_beginning_of_file(&mut self.doc, &mut self.view);
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
            A(SelectMode) => self.selection = Some(self.doc.cur),
            A(ExitSelectMode) => self.selection = None,
            A(YankSelection) => {
                match yank::selection(&mut self.doc, &mut self.selection, &mut self.clipboard) {
                    Ok(()) => {}
                    Err(err) => return err,
                }
            }
            A(YankLine) => match yank::line(&mut self.doc, &mut self.view, &mut self.clipboard) {
                Ok(()) => {}
                Err(err) => return err,
            },
            A(YankLeft) => match yank::left(&mut self.doc, &mut self.view, &mut self.clipboard) {
                Ok(()) => {}
                Err(err) => return err,
            },
            A(YankRight) => match yank::right(&mut self.doc, &mut self.view, &mut self.clipboard) {
                Ok(()) => {}
                Err(err) => return err,
            },
            A(YankNextWord) => {
                match yank::next_word(&mut self.doc, &mut self.view, &mut self.clipboard) {
                    Ok(()) => {}
                    Err(err) => return err,
                }
            }
            A(YankPrevWord) => {
                match yank::prev_word(&mut self.doc, &mut self.view, &mut self.clipboard) {
                    Ok(()) => {}
                    Err(err) => return err,
                }
            }
            A(YankToBeginningOfLine) => {
                match yank::beginning_of_line(&mut self.doc, &mut self.view, &mut self.clipboard) {
                    Ok(()) => {}
                    Err(err) => return err,
                }
            }
            A(YankToEndOfLine) => {
                match yank::end_of_line(&mut self.doc, &mut self.view, &mut self.clipboard) {
                    Ok(()) => {}
                    Err(err) => return err,
                }
            }
            A(YankToMatchingOpposite) => {
                match yank::matching_opposite(&mut self.doc, &mut self.view, &mut self.clipboard) {
                    Ok(()) => {}
                    Err(err) => return err,
                }
            }
            A(YankToBeginningOfFile) => {
                match yank::beginning_of_file(&mut self.doc, &mut self.view, &mut self.clipboard) {
                    Ok(()) => {}
                    Err(err) => return err,
                }
            }
            A(YankToEndOfFile) => {
                match yank::end_of_file(&mut self.doc, &mut self.view, &mut self.clipboard) {
                    Ok(()) => {}
                    Err(err) => return err,
                }
            }
            A(ChangeToTextBuffer) => return CommandResult::ChangeBuffer(TXT_BUFF_IDX),
            A(ChangeToInfoBuffer) => return CommandResult::ChangeBuffer(INFO_BUFF_IDX),
            A(Repeat(ch)) => {
                self.motion_repeat.push(ch);

                // Skip resetting motion repeat buffer when new repeat was issued.
                return CommandResult::Ok;
            }
            Incomplete => return CommandResult::Ok,
            Invalid => {}
        }

        // Rest motion repeat buffer after successful command.
        self.motion_repeat.clear();
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
