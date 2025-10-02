mod apply_command;
mod edit;

use crate::{
    FILES_BUFF_IDX, INFO_BUFF_IDX,
    buffer::{Buffer, delete, yank},
    cursor::{self, Cursor},
    document::Document,
    state_machine::{ChainResult, CommandMap, StateMachine},
    util::{CommandResult, CursorStyle, open_file, read_file_to_lines},
    viewport::Viewport,
};
use arboard::Clipboard;
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
    ChangeSelection,
    ChangeLine,
    ChangeLeft,
    ChangeRight,
    ChangeNextWord,
    ChangePrevWord,
    ChangeToBeginningOfLine,
    ChangeToEndOfLine,
    ChangeToMatchingOpposite,
    ChangeToBeginningOfFile,
    ChangeToEndOfFile,
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
    Paste,
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
    sel: Option<Cursor>,
    mode: Mode,
    motion_repeat: String,
    clipboard: Clipboard,
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
            #[allow(clippy::enum_glob_use)]
            use ViewAction::*;

            let command_map = CommandMap::new()
                .simple(Key::Char('i'), Insert)
                .simple(Key::Char('a'), Append)
                .simple(Key::Char('A'), AppendEndOfLine)
                .simple(Key::Char('o'), InsertBellow)
                .simple(Key::Char('O'), InsertAbove)
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
                .simple(Key::Char('?'), ChangeToInfoBuffer)
                .simple(Key::Char('e'), ChangeToFilesBuffer)
                .simple(Key::Char(' '), CommandMode)
                .simple(Key::Char('v'), SelectMode)
                .simple(Key::Esc, ExitSelectMode)
                .operator(Key::Char('d'), |key| match key {
                    Key::Char('v') => Some(ChainResult::Action(DeleteSelection)),
                    Key::Char('d') => Some(ChainResult::Action(DeleteLine)),
                    Key::Char('h') => Some(ChainResult::Action(DeleteLeft)),
                    Key::Char('l') => Some(ChainResult::Action(DeleteRight)),
                    Key::Char('w') => Some(ChainResult::Action(DeleteNextWord)),
                    Key::Char('b') => Some(ChainResult::Action(DeletePrevWord)),
                    Key::Char('<') => Some(ChainResult::Action(DeleteToBeginningOfLine)),
                    Key::Char('>') => Some(ChainResult::Action(DeleteToEndOfLine)),
                    Key::Char('.') => Some(ChainResult::Action(DeleteToMatchingOpposite)),
                    Key::Char('g') => Some(ChainResult::Action(DeleteToEndOfFile)),
                    Key::Char('G') => Some(ChainResult::Action(DeleteToBeginningOfFile)),
                    _ => None,
                })
                .simple(Key::Char('x'), DeleteRight)
                .operator(Key::Char('c'), |key| match key {
                    Key::Char('v') => Some(ChainResult::Action(ChangeSelection)),
                    Key::Char('c') => Some(ChainResult::Action(ChangeLine)),
                    Key::Char('h') => Some(ChainResult::Action(ChangeLeft)),
                    Key::Char('l') => Some(ChainResult::Action(ChangeRight)),
                    Key::Char('w') => Some(ChainResult::Action(ChangeNextWord)),
                    Key::Char('b') => Some(ChainResult::Action(ChangePrevWord)),
                    Key::Char('<') => Some(ChainResult::Action(ChangeToBeginningOfLine)),
                    Key::Char('>') => Some(ChainResult::Action(ChangeToEndOfLine)),
                    Key::Char('.') => Some(ChainResult::Action(ChangeToMatchingOpposite)),
                    Key::Char('g') => Some(ChainResult::Action(ChangeToEndOfFile)),
                    Key::Char('G') => Some(ChainResult::Action(ChangeToBeginningOfFile)),
                    _ => None,
                })
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
                .simple(Key::Char('p'), Paste)
                .prefix(Key::Char('r'), |key| match key {
                    Key::Char(ch) => Some(ChainResult::Action(ReplaceChar(ch))),
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

        let write_state_machine = {
            #[allow(clippy::enum_glob_use)]
            use WriteAction::*;

            let command_map = CommandMap::new()
                .simple(Key::Esc, ViewMode)
                .simple(Key::Left, Left)
                .simple(Key::Down, Down)
                .simple(Key::Up, Up)
                .simple(Key::Right, Right)
                .simple(Key::AltRight, NextWord)
                .simple(Key::AltLeft, PrevWord)
                .simple(Key::Char('\n'), Newline)
                .simple(Key::Char('\t'), Tab)
                .simple(Key::Backspace, DeleteChar);
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        let cmd_state_machine = {
            #[allow(clippy::enum_glob_use)]
            use CommandAction::*;

            let command_map = CommandMap::new()
                .simple(Key::Esc, ViewMode)
                .simple(Key::Left, Left)
                .simple(Key::Right, Right)
                .simple(Key::Char('\n'), Newline)
                .simple(Key::Char('\t'), Tab)
                .simple(Key::Backspace, DeleteChar);
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        let count = if let Some(content) = &content {
            content.len()
        } else {
            1
        };
        Ok(TextBuffer {
            doc: Document::new(0, 0, content),
            cmd: Document::new(0, 0, None),
            view: Viewport::new(w, h, 0, h / 2, count),
            file,
            sel: None,
            mode: Mode::View,
            motion_repeat: String::new(),
            clipboard: Clipboard::new().map_err(Error::other)?,
            view_state_machine,
            write_state_machine,
            cmd_state_machine,
        })
    }

    fn info_line(&mut self) {
        use std::fmt::Write;

        self.view.info_line.clear();

        let mode = match self.mode {
            Mode::View => "V",
            Mode::Write => "W",
            Mode::Command => "C",
        };
        // Plus 1 since text coordinates are 0 indexed.
        let line = self.doc.cur.y + 1;
        let col = self.doc.cur.x + 1;
        let total = self.doc.buff.len();
        let percentage = 100 * line / total;
        let size: usize = self.doc.buff.iter().map(|l| l.len()).sum();

        write!(
            &mut self.view.info_line,
            "[Text] [{mode}] [{line}:{col}/{total} {percentage}%] [{size}B]",
        )
        .unwrap();
        if let Some(pos) = self.sel {
            // Plus 1 since text coordinates are 0 indexed.
            let line = pos.y + 1;
            let col = pos.x + 1;
            write!(
                &mut self.view.info_line,
                " [Selected {line}:{col} - {}:{}]",
                self.doc.cur.y + 1,
                self.doc.cur.x + 1
            )
            .unwrap();
        }

        let edited = if self.doc.edited { '*' } else { ' ' };
        write!(&mut self.view.info_line, " {edited}").unwrap();
    }

    fn cmd_line(&self) -> Option<(String, Cursor)> {
        match self.mode {
            Mode::Command => Some((self.cmd.buff[0].to_string(), self.cmd.cur)),
            _ => None,
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
            Mode::View | Mode::Write => {}
        }

        match mode {
            Mode::Command => {
                // Set cursor to the beginning of line to avoid weird scrolling behaviour.
                // TODO: save curr position and restore.
                cursor::jump_to_beginning_of_line(&mut self.doc, &mut self.view);
            }
            Mode::View | Mode::Write => {}
        }

        self.mode = mode;
    }

    fn view_tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use ViewAction::*;

        match self.view_state_machine.tick(key.into()) {
            A(Insert) => self.change_mode(Mode::Write),
            A(Append) => {
                cursor::right(&mut self.doc, &mut self.view, 1);
                self.change_mode(Mode::Write);
            }
            A(AppendEndOfLine) => {
                cursor::jump_to_end_of_line(&mut self.doc, &mut self.view);
                self.change_mode(Mode::Write);
            }
            A(InsertBellow) => {
                self.insert_move_new_line_bellow();
                self.change_mode(Mode::Write);
            }
            A(InsertAbove) => {
                self.insert_move_new_line_above();
                self.change_mode(Mode::Write);
            }
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
            A(ChangeToInfoBuffer) => return CommandResult::ChangeBuffer(INFO_BUFF_IDX),
            A(ChangeToFilesBuffer) => {
                return CommandResult::ChangeBuffer(FILES_BUFF_IDX);
            }
            A(CommandMode) => self.change_mode(Mode::Command),
            A(SelectMode) => self.sel = Some(self.doc.cur),
            A(ExitSelectMode) => self.sel = None,
            A(DeleteSelection) => {
                delete::selection(&mut self.doc, &mut self.view, &mut self.sel);
            }
            A(DeleteLine) => delete::line(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(DeleteLeft) => delete::left(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(DeleteRight) => delete::right(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(DeleteNextWord) => delete::next_word(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(DeletePrevWord) => delete::prev_word(
                &mut self.doc,
                &mut self.view,
                self.motion_repeat.parse::<usize>().unwrap_or(1),
            ),
            A(DeleteToBeginningOfLine) => delete::beginning_of_line(&mut self.doc, &mut self.view),
            A(DeleteToEndOfLine) => delete::end_of_line(&mut self.doc, &mut self.view),
            A(DeleteToMatchingOpposite) => delete::matching_opposite(&mut self.doc, &mut self.view),
            A(DeleteToBeginningOfFile) => delete::beginning_of_file(&mut self.doc, &mut self.view),
            A(DeleteToEndOfFile) => delete::end_of_file(&mut self.doc, &mut self.view),
            A(ChangeSelection) => {
                delete::selection(&mut self.doc, &mut self.view, &mut self.sel);
                self.change_mode(Mode::Write);
            }
            A(ChangeLine) => {
                cursor::jump_to_beginning_of_line(&mut self.doc, &mut self.view);
                self.doc.buff[self.doc.cur.y].to_mut().clear();
                self.change_mode(Mode::Write);
            }
            A(ChangeLeft) => {
                delete::left(
                    &mut self.doc,
                    &mut self.view,
                    self.motion_repeat.parse::<usize>().unwrap_or(1),
                );
                self.change_mode(Mode::Write);
            }
            A(ChangeRight) => {
                delete::right(
                    &mut self.doc,
                    &mut self.view,
                    self.motion_repeat.parse::<usize>().unwrap_or(1),
                );
                self.change_mode(Mode::Write);
            }
            A(ChangeNextWord) => {
                delete::next_word(
                    &mut self.doc,
                    &mut self.view,
                    self.motion_repeat.parse::<usize>().unwrap_or(1),
                );
                self.change_mode(Mode::Write);
            }
            A(ChangePrevWord) => {
                delete::prev_word(
                    &mut self.doc,
                    &mut self.view,
                    self.motion_repeat.parse::<usize>().unwrap_or(1),
                );
                self.change_mode(Mode::Write);
            }
            A(ChangeToBeginningOfLine) => {
                delete::beginning_of_line(&mut self.doc, &mut self.view);
                self.change_mode(Mode::Write);
            }
            A(ChangeToEndOfLine) => {
                delete::end_of_line(&mut self.doc, &mut self.view);
                self.change_mode(Mode::Write);
            }
            A(ChangeToMatchingOpposite) => {
                delete::matching_opposite(&mut self.doc, &mut self.view);
                self.change_mode(Mode::Write);
            }
            A(ChangeToBeginningOfFile) => {
                delete::beginning_of_file(&mut self.doc, &mut self.view);
                self.change_mode(Mode::Write);
            }
            A(ChangeToEndOfFile) => {
                delete::end_of_file(&mut self.doc, &mut self.view);
                self.change_mode(Mode::Write);
            }
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
            A(Paste) => {
                let content = match self.clipboard.get_text() {
                    Ok(content) => content,
                    Err(err) => {
                        return CommandResult::SetAndChangeBuffer(
                            INFO_BUFF_IDX,
                            vec![Cow::from(err.to_string())],
                            None,
                        );
                    }
                };

                self.doc.write_str(&content);
            }
            A(ReplaceChar(ch)) => {
                if self.doc.buff[self.doc.cur.y]
                    .chars()
                    .nth(self.doc.cur.x)
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

    fn write_tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use WriteAction::*;

        match self.write_state_machine.tick(key.into()) {
            A(ViewMode) => self.change_mode(Mode::View),
            A(Left) => cursor::left(&mut self.doc, &mut self.view, 1),
            A(Down) => cursor::down(&mut self.doc, &mut self.view, 1),
            A(Up) => cursor::up(&mut self.doc, &mut self.view, 1),
            A(Right) => cursor::right(&mut self.doc, &mut self.view, 1),
            A(NextWord) => cursor::next_word(&mut self.doc, &mut self.view, 1),
            A(PrevWord) => cursor::prev_word(&mut self.doc, &mut self.view, 1),
            A(Newline) => self.write_new_line_char(),
            A(Tab) => self.write_tab(),
            A(DeleteChar) => self.delete_char(),
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
        #[allow(clippy::enum_glob_use)]
        use CommandAction::*;

        match self.cmd_state_machine.tick(key.into()) {
            A(ViewMode) => self.change_mode(Mode::View),
            A(Left) => cursor::left(&mut self.cmd, &mut self.view, 1),
            A(Right) => cursor::right(&mut self.cmd, &mut self.view, 1),
            A(Newline) => {
                let res = self.apply_command();
                self.change_mode(Mode::View);
                return res;
            }
            A(Tab) => self.write_cmd_tab(),
            A(DeleteChar) => self.delete_cmd_char(),
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
        self.info_line();

        let cursor_style = match self.mode {
            Mode::View => CursorStyle::BlinkingBlock,
            Mode::Write | Mode::Command => CursorStyle::BlinkingBar,
        };
        self.view.cmd = self.cmd_line();
        self.view.render(stdout, &self.doc, self.sel, cursor_style)
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.buff_w == w && self.view.buff_h == h {
            return;
        }

        self.view
            .resize(w, h, self.view.cur.x.min(w), h / 2, self.doc.buff.len());
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
