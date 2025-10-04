use crate::{
    buffer::{edit, yank},
    cursor::{self, Cursor},
    document::Document,
    state_machine::{ChainResult, CommandMap, StateMachine},
    util::CommandResult,
    viewport::Viewport,
};
use arboard::Clipboard;
use std::{borrow::Cow, io::Error, time::Duration};
use termion::event::Key;

macro_rules! movement {
    ($self:ident, $func:ident) => {
        cursor::$func(
            &mut $self.doc,
            &mut $self.view,
            $self.motion_repeat.parse::<usize>().unwrap_or(1),
        )
    };
}

macro_rules! jump {
    ($self:ident, $func:ident) => {
        cursor::$func(&mut $self.doc, &mut $self.view)
    };
}

macro_rules! yank {
    ($self:ident, $func:ident) => {
        match yank::$func(&mut $self.doc, &mut $self.view, &mut $self.clipboard) {
            Ok(()) => {}
            Err(err) => {
                $self.motion_repeat.clear();
                return Ok(err);
            }
        }
    };
    ($self:ident, $func:ident, SELECTION) => {
        match yank::$func(&mut $self.doc, &mut $self.sel, &mut $self.clipboard) {
            Ok(()) => {}
            Err(err) => {
                $self.motion_repeat.clear();
                return Ok(err);
            }
        }
    };
}

#[derive(Clone, Copy)]
pub enum ViewAction<T> {
    // Movement
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

    // Modes
    SelectMode,
    ExitSelectMode,
    CommandMode,
    YankToEndOfFile,

    // Yank
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

    // Repeat
    Repeat(char),

    // Other
    Other(T),
}

#[derive(Clone, Copy)]
pub enum CommandAction<T> {
    // Movement
    Left,
    Right,
    NextWord,
    PrevWord,

    // Modes
    ViewMode,

    // Input
    Tab,
    Newline,
    DeleteChar,

    // Other
    Other(T),
}

#[derive(Clone, Copy)]
pub enum CommandTick<T> {
    Apply,
    Other(T),
}

#[derive(Clone, Copy)]
pub enum Mode<T> {
    View,
    Command,
    Other(T),
}

pub struct BaseBuffer<ModeEnum: Clone, ViewEnum: Clone, CommandEnum: Clone> {
    pub doc: Document,
    pub cmd: Document,
    pub view: Viewport,

    pub sel: Option<Cursor>,
    pub mode: Mode<ModeEnum>,
    pub motion_repeat: String,
    pub clipboard: Clipboard,

    pub view_state_machine: StateMachine<ViewAction<ViewEnum>>,
    pub cmd_state_machine: StateMachine<CommandAction<CommandEnum>>,

    pub rerender: bool,
}

impl<ModeEnum: Clone, ViewEnum: Clone, CommandEnum: Clone>
    BaseBuffer<ModeEnum, ViewEnum, CommandEnum>
{
    pub fn new(
        w: usize,
        h: usize,
        count: usize,
        contents: Option<Vec<Cow<'static, str>>>,
    ) -> Result<Self, Error> {
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

        Ok(BaseBuffer {
            doc: Document::new(0, 0, contents),
            cmd: Document::new(0, 0, None),
            view: Viewport::new(w, h, 0, 0, count),
            sel: None,
            mode: Mode::View,
            motion_repeat: String::new(),
            clipboard: Clipboard::new().map_err(Error::other)?,
            view_state_machine,
            cmd_state_machine,
            rerender: true,
        })
    }

    /// Returns the command line string and cursor position.
    pub fn command_line(&mut self) -> Option<(String, Cursor)> {
        match self.mode {
            Mode::Command => Some((self.cmd.buff[0].to_string(), self.cmd.cur)),
            _ => None,
        }
    }

    /// Changes the base buffers mode.
    pub fn change_mode(&mut self, mode: Mode<ModeEnum>) {
        match self.mode {
            Mode::Command => {
                // Clear command line so its ready for next entry.
                self.cmd.buff[0].to_mut().clear();

                // Set cursor to the beginning of line so its always at a predictable position.
                // TODO: restore prev position.
                cursor::left(&mut self.doc, &mut self.view, self.cmd.cur.x);

                self.cmd.cur = Cursor::new(0, 0);
            }
            Mode::View | Mode::Other(_) => {}
        }

        match mode {
            Mode::Command => {
                // Set cursor to the beginning of line to avoid weird scrolling behaviour.
                // TODO: save curr position and restore.
                cursor::jump_to_beginning_of_line(&mut self.doc, &mut self.view);
            }
            Mode::View | Mode::Other(_) => {}
        }

        self.mode = mode;
    }

    /// Handles the tick event in view mode. If a non-standard view action is invoked Err(Other(_)) is returned.
    pub fn view_tick(&mut self, key: Option<Key>) -> Result<CommandResult, ViewEnum> {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use ViewAction::*;

        // Only rerender if input was received.
        self.rerender |= key.is_some();
        match self.view_state_machine.tick(key.into()) {
            A(Left) => movement!(self, left),
            A(Down) => movement!(self, down),
            A(Up) => movement!(self, up),
            A(Right) => movement!(self, right),
            A(NextWord) => movement!(self, next_word),
            A(PrevWord) => movement!(self, prev_word),
            A(JumpToBeginningOfLine) => jump!(self, jump_to_beginning_of_line),
            A(JumpToEndOfLine) => jump!(self, jump_to_end_of_line),
            A(JumpToMatchingOpposite) => jump!(self, jump_to_matching_opposite),
            A(JumpToEndOfFile) => jump!(self, jump_to_end_of_file),
            A(JumpToBeginningOfFile) => jump!(self, jump_to_beginning_of_file),
            A(SelectMode) => self.sel = Some(self.doc.cur),
            A(ExitSelectMode) => self.sel = None,
            A(YankSelection) => yank!(self, selection, SELECTION),
            A(YankLine) => yank!(self, line),
            A(YankLeft) => yank!(self, left),
            A(YankRight) => yank!(self, right),
            A(YankNextWord) => yank!(self, next_word),
            A(YankPrevWord) => yank!(self, prev_word),
            A(YankToBeginningOfLine) => yank!(self, beginning_of_line),
            A(YankToEndOfLine) => yank!(self, end_of_line),
            A(YankToMatchingOpposite) => yank!(self, matching_opposite),
            A(YankToEndOfFile) => yank!(self, end_of_file),
            A(YankToBeginningOfFile) => yank!(self, beginning_of_file),
            A(CommandMode) => self.change_mode(Mode::Command),
            A(Repeat(ch)) => {
                self.motion_repeat.push(ch);

                // Skip resetting motion repeat buffer when new repeat was issued.
                return Ok(CommandResult::Ok);
            }
            // Don't clear motion repeat buffer since the Other(_) handler might need to use it.
            // The Other(_) handler must clear it if it needs clearing.
            A(Other(action)) => return Err(action),
            Incomplete => return Ok(CommandResult::Ok),
            Invalid => {}
        }

        // Rest motion repeat buffer after successful command.
        self.motion_repeat.clear();
        Ok(CommandResult::Ok)
    }

    /// Handles the tick event in command mode. If the apply command is invoked, Err(Apply) is returned, if a
    /// non-standard command action is invoked Err(Other(_)) is returned.
    pub fn command_tick(
        &mut self,
        key: Option<Key>,
    ) -> Result<CommandResult, CommandTick<CommandEnum>> {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use CommandAction::*;

        match self.cmd_state_machine.tick(key.into()) {
            A(ViewMode) => self.change_mode(Mode::View),
            A(Left) => cursor::left(&mut self.cmd, &mut self.view, 1),
            A(Right) => cursor::right(&mut self.cmd, &mut self.view, 1),
            A(NextWord) => cursor::next_word(&mut self.cmd, &mut self.view, 1),
            A(PrevWord) => cursor::prev_word(&mut self.cmd, &mut self.view, 1),
            A(Newline) => return Err(CommandTick::Apply),
            A(Tab) => edit::write_tab(&mut self.cmd, &mut self.view),
            A(DeleteChar) => edit::delete_char(&mut self.cmd, &mut self.view),
            A(Other(tick)) => return Err(CommandTick::Other(tick)),
            Invalid => {
                if let Some(Key::Char(ch)) = key {
                    edit::write_char(&mut self.cmd, &mut self.view, ch);
                }
            }
            Incomplete => {}
        }

        Ok(CommandResult::Ok)
    }
}
