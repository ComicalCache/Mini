mod apply_command;

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
            &mut $self.doc_view,
            $self.motion_repeat.parse::<usize>().unwrap_or(1),
        )
    };
}

macro_rules! jump {
    ($self:ident, $func:ident) => {
        cursor::$func(&mut $self.doc, &mut $self.doc_view)
    };
}

macro_rules! yank {
    ($self:ident, $func:ident) => {
        match yank::$func(&mut $self.doc, &mut $self.doc_view, &mut $self.clipboard) {
            Ok(()) => {}
            Err(err) => {
                $self.motion_repeat.clear();
                return Ok(err);
            }
        }
    };
    ($self:ident, $func:ident, REPEAT) => {{
        let res = yank::$func(
            &mut $self.doc,
            &mut $self.doc_view,
            &mut $self.clipboard,
            $self.motion_repeat.parse::<usize>().unwrap_or(1),
        );
        $self.motion_repeat.clear();

        match res {
            Ok(()) => {}
            Err(err) => return Ok(err),
        }
    }};
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

pub const COMMAND_PROMPT: &str = " > ";

#[derive(Clone, Copy)]
/// A base set of actions in the view mode.
pub enum ViewAction<T> {
    // Movement.
    Left,
    ShiftLeft,
    Down,
    ShiftDown,
    Up,
    ShiftUp,
    Right,
    ShiftRight,
    NextWord,
    NextWordEnd,
    PrevWord,
    PrevWordEnd,
    NextWhitespace,
    PrevWhitespace,
    NextEmptyLine,
    PrevEmptyLine,
    JumpToBeginningOfLine,
    JumpToEndOfLine,
    JumpToMatchingOpposite,
    JumpToBeginningOfFile,
    JumpToEndOfFile,

    // Modes.
    SelectMode,
    ExitSelectMode,
    CommandMode,
    YankToEndOfFile,

    // Yank.
    YankSelection,
    YankLine,
    YankLeft,
    YankRight,
    YankNextWord,
    YankPrevWord,
    YankNextWordEnd,
    YankPrevWordEnd,
    YankNextWhitespace,
    YankPrevWhitespace,
    YankNextEmptyLine,
    YankPrevEmptyLine,
    YankToBeginningOfLine,
    YankToEndOfLine,
    YankToMatchingOpposite,
    YankToBeginningOfFile,

    // Regex.
    NextMatch,
    PrevMatch,

    // Repeat.
    Repeat(char),

    /// Other actions defined by specialized buffers.
    Other(T),
}

#[derive(Clone, Copy)]
/// A base set of actions in the command mode.
pub enum CommandAction<T> {
    // Movement.
    Left,
    Right,
    NextWord,
    PrevWord,

    // Modes.
    ViewMode,

    // Input.
    Tab,
    Newline,
    DeleteChar,

    // History.
    HistoryUp,
    HistoryDown,

    /// Other actions defined by specialized buffers.
    Other(T),
}

#[derive(Clone)]
/// Command mode encountered a non-standard event or can be applied.
pub enum CommandTick<T> {
    /// The command can be applied to the buffer.
    Apply(Cow<'static, str>),

    /// Other actions defined by specialized buffers.
    Other(T),
}

#[derive(Clone, Copy)]
/// A base set of buffer mode.
pub enum Mode<T> {
    /// View mode, made for inspecting a document.
    View,
    /// Command mode, made to issue commands to the buffer.
    Command,
    /// Other modes defined by specialized buffers.
    Other(T),
}

/// A struct defining the base functionality of a buffer. Specialized buffers can keep
/// it as a field to "inherit" this base. Buffers with completely separate functionality
/// can use it as a blueprint and define their own functionality from scratch.
pub struct BaseBuffer<ModeEnum: Clone, ViewEnum: Clone, CommandEnum: Clone> {
    /// The main content of the buffer.
    pub doc: Document,
    /// The command content.
    pub cmd: Document,

    /// The viewport of the document.
    pub doc_view: Viewport,
    /// The viewport of the info bar.
    pub info_view: Viewport,
    /// The viewport of the command line.
    pub cmd_view: Viewport,

    /// Marker of the start of the selection.
    pub sel: Option<Cursor>,
    /// The current buffer mode.
    pub mode: Mode<ModeEnum>,
    /// A buffer to repeat motions.
    pub motion_repeat: String,
    /// An instance of the system clipboard to yank to.
    pub clipboard: Clipboard,

    /// The vector of matches of a search.
    matches: Vec<(Cursor, Cursor)>,
    /// The index of the current match for navigation.
    matches_idx: Option<usize>,

    /// The history of entered commands.
    cmd_history: Vec<Cow<'static, str>>,
    /// The current index in the command history.
    cmd_history_idx: usize,

    /// The state machine handling input in view mode.
    pub view_state_machine: StateMachine<ViewAction<ViewEnum>>,
    /// The state machine handling input in command mode.
    pub cmd_state_machine: StateMachine<CommandAction<CommandEnum>>,

    /// Flag if the buffer needs re-rendering.
    pub rerender: bool,
}

impl<ModeEnum: Clone, ViewEnum: Clone, CommandEnum: Clone>
    BaseBuffer<ModeEnum, ViewEnum, CommandEnum>
{
    pub fn new(
        w: usize,
        h: usize,
        x_off: usize,
        y_off: usize,
        contents: Option<String>,
    ) -> Result<Self, Error> {
        let view_state_machine = {
            #[allow(clippy::enum_glob_use)]
            use ViewAction::*;

            let command_map = CommandMap::new()
                .simple(Key::Char('h'), Left)
                .simple(Key::Char('H'), ShiftLeft)
                .simple(Key::Char('j'), Down)
                .simple(Key::Char('J'), ShiftDown)
                .simple(Key::Char('k'), Up)
                .simple(Key::Char('K'), ShiftUp)
                .simple(Key::Char('l'), Right)
                .simple(Key::Char('L'), ShiftRight)
                .simple(Key::Left, Left)
                .simple(Key::Down, Down)
                .simple(Key::Up, Up)
                .simple(Key::Right, Right)
                .simple(Key::Char('w'), NextWord)
                .simple(Key::Char('W'), NextWordEnd)
                .simple(Key::Char('b'), PrevWord)
                .simple(Key::Char('B'), PrevWordEnd)
                .simple(Key::Char('s'), NextWhitespace)
                .simple(Key::Char('S'), PrevWhitespace)
                .simple(Key::Char('}'), NextEmptyLine)
                .simple(Key::Char('{'), PrevEmptyLine)
                .simple(Key::Char('<'), JumpToBeginningOfLine)
                .simple(Key::Char('>'), JumpToEndOfLine)
                .simple(Key::Char('.'), JumpToMatchingOpposite)
                .simple(Key::Char('g'), JumpToEndOfFile)
                .simple(Key::Char('G'), JumpToBeginningOfFile)
                .simple(Key::Char(' '), CommandMode)
                .simple(Key::Esc, ExitSelectMode)
                .simple(Key::Char('v'), SelectMode)
                .operator(Key::Char('y'), |key| match key {
                    Key::Char('v') => Some(ChainResult::Action(YankSelection)),
                    Key::Char('y') => Some(ChainResult::Action(YankLine)),
                    Key::Char('h') => Some(ChainResult::Action(YankLeft)),
                    Key::Char('l') => Some(ChainResult::Action(YankRight)),
                    Key::Char('w') => Some(ChainResult::Action(YankNextWord)),
                    Key::Char('W') => Some(ChainResult::Action(YankNextWordEnd)),
                    Key::Char('b') => Some(ChainResult::Action(YankPrevWord)),
                    Key::Char('B') => Some(ChainResult::Action(YankPrevWordEnd)),
                    Key::Char('s') => Some(ChainResult::Action(YankNextWhitespace)),
                    Key::Char('S') => Some(ChainResult::Action(YankPrevWhitespace)),
                    Key::Char('}') => Some(ChainResult::Action(YankNextEmptyLine)),
                    Key::Char('{') => Some(ChainResult::Action(YankPrevEmptyLine)),
                    Key::Char('<') => Some(ChainResult::Action(YankToBeginningOfLine)),
                    Key::Char('>') => Some(ChainResult::Action(YankToEndOfLine)),
                    Key::Char('.') => Some(ChainResult::Action(YankToMatchingOpposite)),
                    Key::Char('g') => Some(ChainResult::Action(YankToEndOfFile)),
                    Key::Char('G') => Some(ChainResult::Action(YankToBeginningOfFile)),
                    _ => None,
                })
                .simple(Key::Char('n'), NextMatch)
                .simple(Key::Char('N'), PrevMatch)
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
                .simple(Key::Up, HistoryUp)
                .simple(Key::Down, HistoryDown)
                .simple(Key::AltRight, NextWord)
                .simple(Key::AltLeft, PrevWord)
                .simple(Key::Char('\n'), Newline)
                .simple(Key::Char('\t'), Tab)
                .simple(Key::Backspace, DeleteChar);
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        // Set the command view number width manually.
        // FIXME: this limits the bar to always be exactly one in height.
        let mut cmd_view = Viewport::new(w, 1, 0, 0, x_off, y_off, None);
        cmd_view.set_absolute_gutter_width(COMMAND_PROMPT.len());

        let count = contents.as_ref().map_or(1, |buff| buff.len().max(1));
        Ok(Self {
            doc: Document::new(0, 0, contents),
            cmd: Document::new(0, 0, None),
            // Shifted by one because of info/command line.
            // FIXME: this limits the bar to always be exactly one in height.
            doc_view: Viewport::new(w, h - 1, 0, 0, x_off, y_off + 1, Some(count)),
            // FIXME: this limits the bar to always be exactly one in height.
            info_view: Viewport::new(w, 1, 0, 0, x_off, y_off, None),
            cmd_view,
            sel: None,
            mode: Mode::View,
            motion_repeat: String::new(),
            clipboard: Clipboard::new().map_err(Error::other)?,
            matches: Vec::new(),
            matches_idx: None,
            cmd_history: Vec::new(),
            cmd_history_idx: 0,
            view_state_machine,
            cmd_state_machine,
            rerender: true,
        })
    }

    /// Resizes the viewports of the buffer.
    pub fn resize(&mut self, w: usize, h: usize, x_off: usize, y_off: usize) {
        let doc_count = self.doc.buff.len();

        // Shifted by one because of info/command line.
        // FIXME: this limits the bar to always be exactly one in height.
        self.doc_view
            .resize(w, h - 1, x_off, y_off + 1, Some(doc_count));
        // FIXME: this limits the bar to always be exactly one in height.
        self.info_view.resize(w, 1, x_off, y_off, None);
        // FIXME: this limits the bar to always be exactly one in height.
        self.cmd_view.resize(w, 1, x_off, y_off, None);
        self.cmd_view
            .set_absolute_gutter_width(COMMAND_PROMPT.len());
    }

    /// Jumps to the next search match if any.
    fn next_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        let idx = self.matches_idx.as_mut().unwrap();
        *idx = (*idx + 1) % self.matches.len();

        self.sel = Some(self.matches[*idx].1);
        cursor::move_to(&mut self.doc, &mut self.doc_view, self.matches[*idx].0);
    }

    // Jumps to the previous search match if any.
    fn prev_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        let idx = self.matches_idx.as_mut().unwrap();
        if *idx != 0 {
            *idx -= 1;
        } else {
            *idx = self.matches.len() - 1;
        }

        self.sel = Some(self.matches[*idx].1);
        cursor::move_to(&mut self.doc, &mut self.doc_view, self.matches[*idx].0);
    }

    /// Clears the existing matches of the buffer.
    pub fn clear_matches(&mut self) {
        self.matches.clear();
        self.matches_idx = None;
    }

    /// Loads the next command history item.
    fn next_command_history(&mut self) {
        if self.cmd_history_idx == self.cmd_history.len() {
            return;
        }

        self.cmd_history_idx += 1;
        if self.cmd_history_idx == self.cmd_history.len() {
            self.cmd.buff[0] = Cow::from("");
        } else {
            self.cmd.buff[0].clone_from(&self.cmd_history[self.cmd_history_idx]);
        }

        cursor::jump_to_end_of_line(&mut self.cmd, &mut self.cmd_view);
    }

    /// Loads the previous command history item.
    fn prev_command_history(&mut self) {
        if self.cmd_history_idx == 0 {
            return;
        }

        self.cmd_history_idx -= 1;
        self.cmd.buff[0].clone_from(&self.cmd_history[self.cmd_history_idx]);

        cursor::jump_to_end_of_line(&mut self.cmd, &mut self.cmd_view);
    }

    /// Changes the base buffers mode.
    pub fn change_mode(&mut self, mode: Mode<ModeEnum>) {
        match self.mode {
            Mode::Command => {
                // Clear command line so its ready for next entry. Don't save contents here since they are only
                // saved when hitting enter.
                self.cmd.buff[0].to_mut().clear();
                self.cmd.cur = Cursor::new(0, 0);
                self.cmd_view.cur = Cursor::new(0, 0);
            }
            Mode::View => {
                self.motion_repeat.clear();

                if self.doc.edited {
                    self.clear_matches();
                }
            }
            Mode::Other(_) => {}
        }

        match mode {
            Mode::Command => self.cmd_history_idx = self.cmd_history.len(),
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
            A(ShiftLeft) => movement!(self, shift_left),
            A(Down) => movement!(self, down),
            A(ShiftDown) => movement!(self, shift_down),
            A(Up) => movement!(self, up),
            A(ShiftUp) => movement!(self, shift_up),
            A(Right) => movement!(self, right),
            A(ShiftRight) => movement!(self, shift_right),
            A(NextWord) => movement!(self, next_word),
            A(NextWordEnd) => movement!(self, next_word_end),
            A(PrevWord) => movement!(self, prev_word),
            A(PrevWordEnd) => movement!(self, prev_word_end),
            A(NextWhitespace) => movement!(self, next_whitespace),
            A(PrevWhitespace) => movement!(self, prev_whitespace),
            A(NextEmptyLine) => movement!(self, next_empty_line),
            A(PrevEmptyLine) => movement!(self, prev_empty_line),
            A(JumpToBeginningOfLine) => jump!(self, jump_to_beginning_of_line),
            A(JumpToEndOfLine) => jump!(self, jump_to_end_of_line),
            A(JumpToMatchingOpposite) => jump!(self, jump_to_matching_opposite),
            A(JumpToEndOfFile) => jump!(self, jump_to_end_of_file),
            A(JumpToBeginningOfFile) => jump!(self, jump_to_beginning_of_file),
            A(SelectMode) => self.sel = Some(self.doc.cur),
            A(ExitSelectMode) => self.sel = None,
            A(YankSelection) => yank!(self, selection, SELECTION),
            A(YankLine) => yank!(self, line),
            A(YankLeft) => yank!(self, left, REPEAT),
            A(YankRight) => yank!(self, right, REPEAT),
            A(YankNextWord) => yank!(self, next_word, REPEAT),
            A(YankPrevWord) => yank!(self, prev_word, REPEAT),
            A(YankNextWordEnd) => yank!(self, next_word_end, REPEAT),
            A(YankPrevWordEnd) => yank!(self, prev_word_end, REPEAT),
            A(YankNextWhitespace) => yank!(self, next_whitespace, REPEAT),
            A(YankPrevWhitespace) => yank!(self, prev_whitespace, REPEAT),
            A(YankNextEmptyLine) => yank!(self, next_empty_line, REPEAT),
            A(YankPrevEmptyLine) => yank!(self, prev_empty_line, REPEAT),
            A(YankToBeginningOfLine) => yank!(self, beginning_of_line),
            A(YankToEndOfLine) => yank!(self, end_of_line),
            A(YankToMatchingOpposite) => yank!(self, matching_opposite),
            A(YankToEndOfFile) => yank!(self, end_of_file),
            A(YankToBeginningOfFile) => yank!(self, beginning_of_file),
            A(CommandMode) => self.change_mode(Mode::Command),
            A(NextMatch) => self.next_match(),
            A(PrevMatch) => self.prev_match(),
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
            A(Left) => cursor::left(&mut self.cmd, &mut self.cmd_view, 1),
            A(Right) => cursor::right(&mut self.cmd, &mut self.cmd_view, 1),
            A(HistoryUp) => self.prev_command_history(),
            A(HistoryDown) => self.next_command_history(),
            A(NextWord) => cursor::next_word(&mut self.cmd, &mut self.cmd_view, 1),
            A(PrevWord) => cursor::prev_word(&mut self.cmd, &mut self.cmd_view, 1),
            A(Newline) => {
                // Commands have only one line.
                let cmd = self.cmd.buff[0].clone();
                if !cmd.is_empty() {
                    self.cmd_history.push(cmd.clone());
                }
                self.change_mode(Mode::View);

                return self.apply_command(cmd);
            }
            A(Tab) => edit::write_tab(&mut self.cmd, &mut self.cmd_view, None, false),
            A(DeleteChar) => edit::delete_char(&mut self.cmd, &mut self.cmd_view, None),
            A(Other(tick)) => return Err(CommandTick::Other(tick)),
            Invalid => {
                if let Some(Key::Char(ch)) = key {
                    edit::write_char(&mut self.cmd, &mut self.cmd_view, None, ch);
                }
            }
            Incomplete => {}
        }

        Ok(CommandResult::Ok)
    }
}
