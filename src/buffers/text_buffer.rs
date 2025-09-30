mod apply_cmd;
mod edit;
mod r#move;

use crate::{
    FILES_BUFF_IDX, INFO_BUFF_IDX,
    buffer::Buffer,
    cursor::Cursor,
    document::Document,
    state_machine::{ChainResult, CommandMap, StateMachine},
    util::{CommandResult, CursorStyle, open_file, read_file_to_lines},
    viewport::Viewport,
};
use std::{
    borrow::Cow,
    fs::File,
    io::{BufWriter, Error, Stdout},
    path::PathBuf,
    time::Duration,
};
use termion::{event::Key, raw::RawTerminal};

#[derive(Clone)]
enum ViewAction {
    Insert,
    Append,
    AppendEndOfLine,
    InsertBellow,
    InsertAbove,
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
    ChangeToInfoBuffer,
    ChangeToFilesBuffer,
    CommandMode,
    SelectMode,
    ExitSelectMode,
    DeleteSelection,
    DeleteLine,
    DeleteLeft,
    DeleteRight,
    DeleteNextWord,
    DeletePrevWord,
    DeleteToBeginningOfLine,
    DeleteToEndOfLine,
    DeleteToMatchingOpposite,
    DeleteToBeginningOfFile,
    DeleteToEndOfFile,
    DeleteChar,
    ReplaceChar(char),
    Repeat(char),
}

#[derive(Clone)]
enum WriteAction {
    ViewMode,
    Left,
    Down,
    Up,
    Right,
    NextWord,
    PrevWord,
    Newline,
    Tab,
    DeleteChar,
}

#[derive(Clone)]
enum CommandAction {
    ViewMode,
    Left,
    Right,
    Newline,
    Tab,
    DeleteChar,
}

#[derive(Clone, Copy)]
enum Mode {
    View,
    Write,
    Command,
}

pub struct TextBuffer {
    doc: Document,
    cmd: Document,
    view: Viewport,
    file: Option<File>,
    selected_pos: Option<Cursor>,
    mode: Mode,
    motion_repeat: String,
    view_state_machine: StateMachine<ViewAction>,
    write_state_machine: StateMachine<WriteAction>,
    cmd_state_machine: StateMachine<CommandAction>,
}

impl TextBuffer {
    pub fn new(w: usize, h: usize, mut file: Option<File>) -> Result<Self, Error> {
        let content = if let Some(file) = file.as_mut() {
            Some(read_file_to_lines(file)?)
        } else {
            None
        };

        let view_state_machine = {
            let command_map = CommandMap::new()
                .simple(Key::Char('i'), ViewAction::Insert)
                .simple(Key::Char('a'), ViewAction::Append)
                .simple(Key::Char('A'), ViewAction::AppendEndOfLine)
                .simple(Key::Char('o'), ViewAction::InsertBellow)
                .simple(Key::Char('O'), ViewAction::InsertAbove)
                .simple(Key::Char('h'), ViewAction::Left)
                .simple(Key::Char('j'), ViewAction::Down)
                .simple(Key::Char('k'), ViewAction::Up)
                .simple(Key::Char('l'), ViewAction::Right)
                .simple(Key::Char('w'), ViewAction::NextWord)
                .simple(Key::Char('b'), ViewAction::PrevWord)
                .simple(Key::Char('<'), ViewAction::JumpToBeginningOfLine)
                .simple(Key::Char('>'), ViewAction::JumpToEndOfLine)
                .simple(Key::Char('.'), ViewAction::JumpToMatchingOpposite)
                .simple(Key::Char('g'), ViewAction::JumpToEndOfFile)
                .simple(Key::Char('G'), ViewAction::JumpToBeginningOfFile)
                .simple(Key::Char('?'), ViewAction::ChangeToInfoBuffer)
                .simple(Key::Char('e'), ViewAction::ChangeToFilesBuffer)
                .simple(Key::Char(' '), ViewAction::CommandMode)
                .simple(Key::Char('v'), ViewAction::SelectMode)
                .simple(Key::Esc, ViewAction::ExitSelectMode)
                .operator(Key::Char('d'), |key| match key {
                    Key::Char('v') => Some(ChainResult::Action(ViewAction::DeleteSelection)),
                    Key::Char('d') => Some(ChainResult::Action(ViewAction::DeleteLine)),
                    Key::Char('h') => Some(ChainResult::Action(ViewAction::DeleteLeft)),
                    Key::Char('l') => Some(ChainResult::Action(ViewAction::DeleteRight)),
                    Key::Char('w') => Some(ChainResult::Action(ViewAction::DeleteNextWord)),
                    Key::Char('b') => Some(ChainResult::Action(ViewAction::DeletePrevWord)),
                    Key::Char('<') => {
                        Some(ChainResult::Action(ViewAction::DeleteToBeginningOfLine))
                    }
                    Key::Char('>') => Some(ChainResult::Action(ViewAction::DeleteToEndOfLine)),
                    Key::Char('.') => {
                        Some(ChainResult::Action(ViewAction::DeleteToMatchingOpposite))
                    }
                    Key::Char('g') => Some(ChainResult::Action(ViewAction::DeleteToEndOfFile)),
                    Key::Char('G') => {
                        Some(ChainResult::Action(ViewAction::DeleteToBeginningOfFile))
                    }
                    _ => None,
                })
                .simple(Key::Char('x'), ViewAction::DeleteChar)
                .prefix(Key::Char('r'), |key| match key {
                    Key::Char(ch) => Some(ChainResult::Action(ViewAction::ReplaceChar(ch))),
                    _ => None,
                })
                .simple(Key::Char('0'), ViewAction::Repeat('0'))
                .simple(Key::Char('1'), ViewAction::Repeat('1'))
                .simple(Key::Char('2'), ViewAction::Repeat('2'))
                .simple(Key::Char('3'), ViewAction::Repeat('3'))
                .simple(Key::Char('4'), ViewAction::Repeat('4'))
                .simple(Key::Char('5'), ViewAction::Repeat('5'))
                .simple(Key::Char('6'), ViewAction::Repeat('6'))
                .simple(Key::Char('7'), ViewAction::Repeat('7'))
                .simple(Key::Char('8'), ViewAction::Repeat('8'))
                .simple(Key::Char('9'), ViewAction::Repeat('9'));
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        let write_state_machine = {
            let command_map = CommandMap::new()
                .simple(Key::Esc, WriteAction::ViewMode)
                .simple(Key::Left, WriteAction::Left)
                .simple(Key::Down, WriteAction::Down)
                .simple(Key::Up, WriteAction::Up)
                .simple(Key::Right, WriteAction::Right)
                .simple(Key::AltRight, WriteAction::NextWord)
                .simple(Key::AltLeft, WriteAction::PrevWord)
                .simple(Key::Char('\n'), WriteAction::Newline)
                .simple(Key::Char('\t'), WriteAction::Tab)
                .simple(Key::Backspace, WriteAction::DeleteChar);
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        let cmd_state_machine = {
            let command_map = CommandMap::new()
                .simple(Key::Esc, CommandAction::ViewMode)
                .simple(Key::Left, CommandAction::Left)
                .simple(Key::Right, CommandAction::Right)
                .simple(Key::Char('\n'), CommandAction::Newline)
                .simple(Key::Char('\t'), CommandAction::Tab)
                .simple(Key::Backspace, CommandAction::DeleteChar);
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        Ok(TextBuffer {
            doc: Document::new(content, 0, 0),
            cmd: Document::new(None, 0, 0),
            view: Viewport::new(w, h, 0, h / 2),
            file,
            selected_pos: None,
            mode: Mode::View,
            motion_repeat: String::new(),
            view_state_machine,
            write_state_machine,
            cmd_state_machine,
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
        let size: usize = self.doc.lines.iter().map(|l| l.len()).sum();

        write!(
            &mut info_line,
            "[Text] [{mode}] [{line}:{col}/{total} {percentage}%] [{size}B]",
        )
        .unwrap();
        if let Some(pos) = self.selected_pos {
            // Plus 1 since text coordinates are 0 indexed.
            let line = pos.y + 1;
            let col = pos.x + 1;
            write!(&mut info_line, " [Selected {line}:{col}]").unwrap();
        }

        let edited = if self.doc.edited { '*' } else { ' ' };
        write!(&mut info_line, " {edited}").unwrap();

        info_line
    }

    fn cmd_line(&self) -> Option<(String, Cursor)> {
        match self.mode {
            Mode::Command => Some((self.cmd.lines[0].to_string(), self.cmd.cursor)),
            _ => None,
        }
    }

    fn change_mode(&mut self, mode: Mode) {
        match self.mode {
            Mode::Command => {
                // Clear command line so its ready for next entry.
                self.cmd.lines[0].to_mut().clear();

                // Set cursor to the beginning of line so its always at a predictable position.
                // TODO: restore prev position.
                self.left(self.cmd.cursor.x);

                self.cmd.cursor = Cursor::new(0, 0);
            }
            Mode::View | Mode::Write => {}
        }

        match mode {
            Mode::Command => {
                // Set cursor to the beginning of line to avoid weird scrolling behaviour.
                // TODO: save curr position and restore.
                self.jump_to_beginning_of_line();
            }
            Mode::View | Mode::Write => {}
        }

        self.mode = mode;
    }

    fn view_tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};

        match self.view_state_machine.tick(key.into()) {
            A(ViewAction::Insert) => self.change_mode(Mode::Write),
            A(ViewAction::Append) => {
                self.right(1);
                self.change_mode(Mode::Write);
            }
            A(ViewAction::AppendEndOfLine) => {
                self.jump_to_end_of_line();
                self.change_mode(Mode::Write);
            }
            A(ViewAction::InsertBellow) => {
                self.insert_move_new_line_bellow();
                self.change_mode(Mode::Write);
            }
            A(ViewAction::InsertAbove) => {
                self.insert_move_new_line_above();
                self.change_mode(Mode::Write);
            }
            A(ViewAction::Left) => self.left(self.motion_repeat.parse::<usize>().unwrap_or(1)),
            A(ViewAction::Down) => self.down(self.motion_repeat.parse::<usize>().unwrap_or(1)),
            A(ViewAction::Up) => self.up(self.motion_repeat.parse::<usize>().unwrap_or(1)),
            A(ViewAction::Right) => self.right(self.motion_repeat.parse::<usize>().unwrap_or(1)),
            A(ViewAction::NextWord) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.next_word();
                }
            }
            A(ViewAction::PrevWord) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.prev_word();
                }
            }
            A(ViewAction::JumpToBeginningOfLine) => self.jump_to_beginning_of_line(),
            A(ViewAction::JumpToEndOfLine) => self.jump_to_end_of_line(),
            A(ViewAction::JumpToMatchingOpposite) => self.jump_to_matching_opposite(),
            A(ViewAction::JumpToEndOfFile) => self.jump_to_end_of_file(),
            A(ViewAction::JumpToBeginningOfFile) => self.jump_to_beginning_of_file(),
            A(ViewAction::ChangeToInfoBuffer) => return CommandResult::ChangeBuffer(INFO_BUFF_IDX),
            A(ViewAction::ChangeToFilesBuffer) => {
                return CommandResult::ChangeBuffer(FILES_BUFF_IDX);
            }
            A(ViewAction::CommandMode) => self.change_mode(Mode::Command),
            A(ViewAction::SelectMode) => self.selected_pos = Some(self.doc.cursor),
            A(ViewAction::ExitSelectMode) => self.selected_pos = None,
            A(ViewAction::DeleteSelection) => self.delete_selection(),
            A(ViewAction::DeleteLine) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.jump_to_beginning_of_line();
                    self.doc.remove_line();
                    if self.doc.lines.is_empty() {
                        self.doc.insert_line(Cow::from(""));
                    }
                    if self.doc.cursor.y == self.doc.lines.len() {
                        self.up(1);
                    }
                }
            }
            A(ViewAction::DeleteLeft) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.selected_pos = Some(self.doc.cursor);
                    self.left(1);
                    self.delete_selection();
                }
            }
            A(ViewAction::DeleteRight) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.selected_pos = Some(self.doc.cursor);
                    self.right(1);
                    self.delete_selection();
                }
            }
            A(ViewAction::DeleteNextWord) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.selected_pos = Some(self.doc.cursor);
                    self.next_word();
                    self.delete_selection();
                }
            }
            A(ViewAction::DeletePrevWord) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.selected_pos = Some(self.doc.cursor);
                    self.prev_word();
                    self.delete_selection();
                }
            }
            A(ViewAction::DeleteToBeginningOfLine) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.selected_pos = Some(self.doc.cursor);
                    self.jump_to_beginning_of_line();
                    self.delete_selection();
                }
            }
            A(ViewAction::DeleteToEndOfLine) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.selected_pos = Some(self.doc.cursor);
                    self.jump_to_end_of_line();
                    self.delete_selection();
                }
            }
            A(ViewAction::DeleteToMatchingOpposite) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.selected_pos = Some(self.doc.cursor);
                    self.jump_to_matching_opposite();
                    self.delete_selection();
                }
            }
            A(ViewAction::DeleteToBeginningOfFile) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.selected_pos = Some(self.doc.cursor);
                    self.jump_to_beginning_of_file();
                    self.delete_selection();
                }
            }
            A(ViewAction::DeleteToEndOfFile) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    self.selected_pos = Some(self.doc.cursor);
                    self.jump_to_end_of_file();
                    self.delete_selection();
                }
            }
            A(ViewAction::DeleteChar) => {
                for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                    if self.doc.lines[self.doc.cursor.y]
                        .chars()
                        .nth(self.doc.cursor.x)
                        .is_some()
                    {
                        self.doc.delete_char();
                    }
                }
            }
            A(ViewAction::ReplaceChar(ch)) => {
                if self.doc.lines[self.doc.cursor.y]
                    .chars()
                    .nth(self.doc.cursor.x)
                    .is_some()
                {
                    self.doc.delete_char();

                    match ch {
                        '\n' => self.write_new_line_char(),
                        '\t' => self.write_tab(),
                        _ => self.doc.write_char(ch),
                    }
                }
            }
            A(ViewAction::Repeat(ch)) => {
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

    fn write_tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};

        match self.write_state_machine.tick(key.into()) {
            A(WriteAction::ViewMode) => self.change_mode(Mode::View),
            A(WriteAction::Left) => self.left(1),
            A(WriteAction::Down) => self.down(1),
            A(WriteAction::Up) => self.up(1),
            A(WriteAction::Right) => self.right(1),
            A(WriteAction::NextWord) => self.next_word(),
            A(WriteAction::PrevWord) => self.prev_word(),
            A(WriteAction::Newline) => self.write_new_line_char(),
            A(WriteAction::Tab) => self.write_tab(),
            A(WriteAction::DeleteChar) => self.delete_char(),
            Invalid => {
                if let Some(Key::Char(ch)) = key {
                    self.write_char(ch);
                }
            }
            Incomplete => {}
        }

        CommandResult::Ok
    }

    fn command_tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};

        match self.cmd_state_machine.tick(key.into()) {
            A(CommandAction::ViewMode) => self.change_mode(Mode::View),
            A(CommandAction::Left) => self.cmd_left(1),
            A(CommandAction::Right) => self.cmd_right(1),
            A(CommandAction::Newline) => {
                let res = self.apply_cmd();
                self.change_mode(Mode::View);
                return res;
            }
            A(CommandAction::Tab) => self.write_cmd_tab(),
            A(CommandAction::DeleteChar) => self.delete_cmd_char(),
            Invalid => {
                if let Some(Key::Char(ch)) = key {
                    self.write_cmd_char(ch);
                }
            }
            Incomplete => {}
        }

        CommandResult::Ok
    }
}

impl Buffer for TextBuffer {
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        let cursor_style = match self.mode {
            Mode::View => CursorStyle::BlinkingBlock,
            Mode::Write | Mode::Command => CursorStyle::BlinkingBar,
        };

        self.view.render(
            stdout,
            &self.doc,
            &self.info_line(),
            self.cmd_line(),
            cursor_style,
        )
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.w == w && self.view.h == h {
            return;
        }

        self.view.resize(w, h, self.view.cursor.x.min(w), h / 2);
    }

    fn tick(&mut self, key: Option<Key>) -> CommandResult {
        match self.mode {
            Mode::View => self.view_tick(key),
            Mode::Write => self.write_tick(key),
            Mode::Command => self.command_tick(key),
        }
    }

    fn set_contents(&mut self, contents: &[Cow<'static, str>], path: Option<PathBuf>) {
        self.doc.set_contents(contents, 0, 0);
        if let Some(path) = path {
            self.file = open_file(path).ok();
        }
    }

    fn can_quit(&self) -> Result<(), Vec<Cow<'static, str>>> {
        if !self.doc.edited {
            return Ok(());
        }

        Err(vec![Cow::from("There are unsaved changes")])
    }
}
