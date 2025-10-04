mod apply_command;
mod interact;

use crate::{
    INFO_BUFF_IDX, TXT_BUFF_IDX,
    buffer::{Buffer, edit, yank},
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
enum ViewAction {
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
    CommandMode,
    Repeat(char),
}

#[derive(Clone)]
enum CommandAction {
    ViewMode,
    Left,
    Right,
    NextWord,
    PrevWord,
    Newline,
    Tab,
    DeleteChar,
}

#[derive(Clone, Copy)]
enum Mode {
    View,
    Command,
}

pub struct FilesBuffer {
    doc: Document,
    cmd: Document,
    view: Viewport,
    base: PathBuf,
    entries: Vec<PathBuf>,

    sel: Option<Cursor>,
    mode: Mode,
    motion_repeat: String,
    clipboard: Clipboard,

    view_state_machine: StateMachine<ViewAction>,
    cmd_state_machine: StateMachine<CommandAction>,

    rerender: bool,
}

impl FilesBuffer {
    pub fn new(w: usize, h: usize, base: PathBuf) -> Result<Self, Error> {
        let mut entries = Vec::new();
        let mut contents = Vec::new();
        FilesBuffer::load_dir(&base, &mut entries, &mut contents)?;

        let view_state_machine = {
            #[allow(clippy::enum_glob_use)]
            use ViewAction::*;

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
                .simple(Key::Char(' '), CommandMode)
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

        let cmd_state_machine = {
            #[allow(clippy::enum_glob_use)]
            use CommandAction::*;

            let command_map = CommandMap::new()
                .simple(Key::Esc, ViewMode)
                .simple(Key::Left, Left)
                .simple(Key::Right, Right)
                .simple(Key::AltRight, NextWord)
                .simple(Key::AltLeft, PrevWord)
                .simple(Key::Char('\n'), Newline)
                .simple(Key::Char('\t'), Tab)
                .simple(Key::Backspace, DeleteChar);
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        let count = contents.len();
        Ok(FilesBuffer {
            doc: Document::new(0, 0, Some(contents)),
            cmd: Document::new(0, 0, None),
            view: Viewport::new(w, h, 0, 0, count),
            base,
            entries,
            sel: None,
            mode: Mode::View,
            motion_repeat: String::new(),
            clipboard: Clipboard::new().map_err(Error::other)?,
            view_state_machine,
            cmd_state_machine,
            rerender: false,
        })
    }

    fn info_line(&mut self) -> Result<(), std::fmt::Error> {
        use std::fmt::Write;

        self.view.info_line.clear();

        let mode = match self.mode {
            Mode::View => "V",
            Mode::Command => "C",
        };
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
            "[Files] [{mode}] [{curr_type}] [{curr}/{entries} {entries_label}]",
        )?;

        if let Some(pos) = self.sel {
            // Plus 1 since text coordinates are 0 indexed.
            let line = pos.y + 1;
            let col = pos.x + 1;
            write!(
                &mut self.view.info_line,
                " [Selected {line}:{col} - {}:{}]",
                self.doc.cur.y + 1,
                self.doc.cur.x + 1
            )?;
        }

        Ok(())
    }

    fn cmd_line(&self) -> Option<(String, Cursor)> {
        match self.mode {
            Mode::Command => Some((self.cmd.buff[0].to_string(), self.cmd.cur)),
            Mode::View => None,
        }
    }

    fn change_mode(&mut self, mode: Mode) {
        match self.mode {
            Mode::Command => {
                // Clear command line so its ready for next entry.
                self.cmd.buff[0].to_mut().clear();

                // Set cursor to the beginning of line so its always at a predictable position.
                // TODO: restore prev position.
                cursor::left(&mut self.doc, &mut self.view, self.cmd.cur.x);

                self.cmd.cur = Cursor::new(0, 0);
            }
            Mode::View => {}
        }

        match mode {
            Mode::Command => {
                // Set cursor to the beginning of line to avoid weird scrolling behaviour.
                // TODO: save curr position and restore.
                cursor::jump_to_beginning_of_line(&mut self.doc, &mut self.view);
            }
            Mode::View => {}
        }

        self.mode = mode;
    }

    fn view_tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use ViewAction::*;

        // Only rerender if input was received.
        self.rerender |= key.is_some();
        match self.view_state_machine.tick(key.into()) {
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
            A(SelectMode) => self.sel = Some(self.doc.cur),
            A(ExitSelectMode) => self.sel = None,
            A(YankSelection) => {
                match yank::selection(&mut self.doc, &mut self.sel, &mut self.clipboard) {
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
            A(CommandMode) => self.change_mode(Mode::Command),
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

    fn command_tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use CommandAction::*;

        match self.cmd_state_machine.tick(key.into()) {
            A(ViewMode) => self.change_mode(Mode::View),
            A(Left) => cursor::left(&mut self.cmd, &mut self.view, 1),
            A(Right) => cursor::right(&mut self.cmd, &mut self.view, 1),
            A(NextWord) => cursor::next_word(&mut self.cmd, &mut self.view, 1),
            A(PrevWord) => cursor::prev_word(&mut self.cmd, &mut self.view, 1),
            A(Newline) => {
                let res = self.apply_command();
                self.change_mode(Mode::View);
                return res;
            }
            A(Tab) => edit::write_tab(&mut self.cmd, &mut self.view),
            A(DeleteChar) => edit::delete_char(&mut self.cmd, &mut self.view),
            Invalid => {
                if let Some(Key::Char(ch)) = key {
                    edit::write_char(&mut self.cmd, &mut self.view, ch);
                }
            }
            Incomplete => {}
        }

        CommandResult::Ok
    }
}

impl Buffer for FilesBuffer {
    fn need_rerender(&self) -> bool {
        self.rerender
    }

    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        self.rerender = false;

        self.info_line().map_err(Error::other)?;

        let cursor_style = match self.mode {
            Mode::View => CursorStyle::SteadyBlock,
            Mode::Command => CursorStyle::BlinkingBar,
        };
        self.view.cmd = self.cmd_line();
        self.view.render(stdout, &self.doc, self.sel, cursor_style)
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.w == w && self.view.h == h {
            return;
        }

        self.rerender = true;

        self.view.resize(w, h, self.doc.buff.len());
    }

    fn tick(&mut self, key: Option<Key>) -> CommandResult {
        // Only rerender if input was received.
        self.rerender |= key.is_some();
        match self.mode {
            Mode::View => self.view_tick(key),
            Mode::Command => self.command_tick(key),
        }
    }

    fn set_contents(&mut self, _: &[Cow<'static, str>], path: Option<PathBuf>) {
        self.doc.set_contents(&[], 0, 0);
        if let Some(path) = path {
            self.base = path;
        }
        self.view.cur = Cursor::new(0, 0);

        self.sel = None;
        self.motion_repeat.clear();

        self.rerender = true;
    }

    fn can_quit(&self) -> Result<(), Vec<Cow<'static, str>>> {
        Ok(())
    }
}
