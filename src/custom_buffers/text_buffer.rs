mod apply_command;
mod insert;

use crate::{
    FILES_BUFF_IDX, INFO_BUFF_IDX,
    buffer::{
        Buffer,
        base::{BaseBuffer, CommandTick, Mode, ViewAction},
        delete, edit,
    },
    change_buffer,
    cursor::{self, Cursor},
    state_machine::{ChainResult, CommandMap, StateMachine},
    util::{CommandResult, CursorStyle, open_file, read_file_to_lines},
};
use std::{
    borrow::Cow,
    fs::File,
    io::{BufWriter, Error, Stdout},
    path::PathBuf,
    time::Duration,
};
use termion::{event::Key, raw::RawTerminal};

macro_rules! delete {
    ($self:ident, $func:ident) => {
        delete::$func(&mut $self.base.doc, &mut $self.base.view)
    };
    ($self:ident, $func:ident, REPEAT) => {
        delete::$func(
            &mut $self.base.doc,
            &mut $self.base.view,
            $self.base.motion_repeat.parse::<usize>().unwrap_or(1),
        )
    };
    ($self:ident, $func:ident, SELECTION) => {
        delete::$func(
            &mut $self.base.doc,
            &mut $self.base.view,
            &mut $self.base.sel,
        )
    };
}

macro_rules! change {
    ($self:ident, $func:ident) => {{
        delete::$func(&mut $self.base.doc, &mut $self.base.view);
        $self.base.change_mode(Mode::Other(Write));
    }};
    ($self:ident, $func:ident, REPEAT) => {{
        delete::$func(
            &mut $self.base.doc,
            &mut $self.base.view,
            $self.base.motion_repeat.parse::<usize>().unwrap_or(1),
        );
        $self.base.change_mode(Mode::Other(Write));
    }};
}

#[derive(Clone, Copy)]
enum OtherViewAction {
    // Insert
    Insert,
    Append,
    AppendEndOfLine,
    InsertBellow,
    InsertAbove,

    // Buffers
    ChangeToInfoBuffer,
    ChangeToFilesBuffer,

    // Delete
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

    // Change
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
    ReplaceChar(char),

    // Paste
    Paste,
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
enum OtherMode {
    Write,
}

pub struct TextBuffer {
    base: BaseBuffer<OtherMode, OtherViewAction, ()>,
    file: Option<File>,
    write_state_machine: StateMachine<WriteAction>,
}

impl TextBuffer {
    pub fn new(w: usize, h: usize, mut file: Option<File>) -> Result<Self, Error> {
        let contents = if let Some(file) = file.as_mut() {
            Some(read_file_to_lines(file)?)
        } else {
            None
        };

        let count = if let Some(content) = &contents {
            content.len()
        } else {
            1
        };
        let mut base = BaseBuffer::new(w, h, count, contents)?;
        {
            use OtherViewAction::*;
            use ViewAction::Other;

            base.view_state_machine.command_map = base
                .view_state_machine
                .command_map
                .simple(Key::Char('i'), Other(Insert))
                .simple(Key::Char('a'), Other(Append))
                .simple(Key::Char('A'), Other(AppendEndOfLine))
                .simple(Key::Char('o'), Other(InsertBellow))
                .simple(Key::Char('O'), Other(InsertAbove))
                .simple(Key::Char('?'), Other(ChangeToInfoBuffer))
                .simple(Key::Char('e'), Other(ChangeToFilesBuffer))
                .operator(Key::Char('d'), |key| match key {
                    Key::Char('v') => Some(ChainResult::Action(Other(DeleteSelection))),
                    Key::Char('d') => Some(ChainResult::Action(Other(DeleteLine))),
                    Key::Char('h') => Some(ChainResult::Action(Other(DeleteLeft))),
                    Key::Char('l') => Some(ChainResult::Action(Other(DeleteRight))),
                    Key::Char('w') => Some(ChainResult::Action(Other(DeleteNextWord))),
                    Key::Char('b') => Some(ChainResult::Action(Other(DeletePrevWord))),
                    Key::Char('<') => Some(ChainResult::Action(Other(DeleteToBeginningOfLine))),
                    Key::Char('>') => Some(ChainResult::Action(Other(DeleteToEndOfLine))),
                    Key::Char('.') => Some(ChainResult::Action(Other(DeleteToMatchingOpposite))),
                    Key::Char('g') => Some(ChainResult::Action(Other(DeleteToEndOfFile))),
                    Key::Char('G') => Some(ChainResult::Action(Other(DeleteToBeginningOfFile))),
                    _ => None,
                })
                .simple(Key::Char('x'), Other(DeleteRight))
                .operator(Key::Char('c'), |key| match key {
                    Key::Char('v') => Some(ChainResult::Action(Other(ChangeSelection))),
                    Key::Char('c') => Some(ChainResult::Action(Other(ChangeLine))),
                    Key::Char('h') => Some(ChainResult::Action(Other(ChangeLeft))),
                    Key::Char('l') => Some(ChainResult::Action(Other(ChangeRight))),
                    Key::Char('w') => Some(ChainResult::Action(Other(ChangeNextWord))),
                    Key::Char('b') => Some(ChainResult::Action(Other(ChangePrevWord))),
                    Key::Char('<') => Some(ChainResult::Action(Other(ChangeToBeginningOfLine))),
                    Key::Char('>') => Some(ChainResult::Action(Other(ChangeToEndOfLine))),
                    Key::Char('.') => Some(ChainResult::Action(Other(ChangeToMatchingOpposite))),
                    Key::Char('g') => Some(ChainResult::Action(Other(ChangeToEndOfFile))),
                    Key::Char('G') => Some(ChainResult::Action(Other(ChangeToBeginningOfFile))),
                    _ => None,
                })
                .simple(Key::Char('p'), Other(Paste))
                .prefix(Key::Char('r'), |key| match key {
                    Key::Char(ch) => Some(ChainResult::Action(Other(ReplaceChar(ch)))),
                    _ => None,
                });
        }

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

        Ok(TextBuffer {
            base,
            file,
            write_state_machine,
        })
    }

    fn info_line(&mut self) -> Result<(), std::fmt::Error> {
        use std::fmt::Write;

        self.base.view.info_line.clear();

        let mode = match self.base.mode {
            Mode::View => "V",
            Mode::Command => "C",
            Mode::Other(OtherMode::Write) => "W",
        };
        // Plus 1 since text coordinates are 0 indexed.
        let line = self.base.doc.cur.y + 1;
        let col = self.base.doc.cur.x + 1;
        let total = self.base.doc.buff.len();
        let percentage = 100 * line / total;
        let size: usize = self.base.doc.buff.iter().map(|l| l.len()).sum();

        write!(
            &mut self.base.view.info_line,
            "[Text] [{mode}] [{line}:{col}/{total} {percentage}%] [{size}B]",
        )?;
        if let Some(pos) = self.base.sel {
            // Plus 1 since text coordinates are 0 indexed.
            let line = pos.y + 1;
            let col = pos.x + 1;
            write!(
                &mut self.base.view.info_line,
                " [Selected {line}:{col} - {}:{}]",
                self.base.doc.cur.y + 1,
                self.base.doc.cur.x + 1
            )?;
        }

        let edited = if self.base.doc.edited { '*' } else { ' ' };
        write!(&mut self.base.view.info_line, " {edited}")?;

        Ok(())
    }

    fn view_action(&mut self, action: OtherViewAction) -> CommandResult {
        use OtherMode::Write;
        use OtherViewAction::*;

        match action {
            Insert => self.base.change_mode(Mode::Other(Write)),
            Append => {
                cursor::right(&mut self.base.doc, &mut self.base.view, 1);
                self.base.change_mode(Mode::Other(Write));
            }
            AppendEndOfLine => {
                cursor::jump_to_end_of_line(&mut self.base.doc, &mut self.base.view);
                self.base.change_mode(Mode::Other(Write));
            }
            InsertBellow => {
                self.insert_move_new_line_bellow();
                self.base.change_mode(Mode::Other(Write));
            }
            InsertAbove => {
                self.insert_move_new_line_above();
                self.base.change_mode(Mode::Other(Write));
            }
            ChangeToInfoBuffer => change_buffer!(self, INFO_BUFF_IDX),
            ChangeToFilesBuffer => change_buffer!(self, FILES_BUFF_IDX),
            DeleteSelection => delete!(self, selection, SELECTION),
            DeleteLine => delete!(self, line, REPEAT),
            DeleteLeft => delete!(self, left, REPEAT),
            DeleteRight => delete!(self, right, REPEAT),
            DeleteNextWord => delete!(self, next_word, REPEAT),
            DeletePrevWord => delete!(self, prev_word, REPEAT),
            DeleteToBeginningOfLine => delete!(self, beginning_of_line),
            DeleteToEndOfLine => delete!(self, end_of_line),
            DeleteToMatchingOpposite => delete!(self, matching_opposite),
            DeleteToEndOfFile => delete!(self, end_of_file),
            DeleteToBeginningOfFile => delete!(self, beginning_of_file),
            ChangeSelection => {
                delete::selection(&mut self.base.doc, &mut self.base.view, &mut self.base.sel);
                self.base.change_mode(Mode::Other(Write));
            }
            ChangeLine => {
                cursor::jump_to_beginning_of_line(&mut self.base.doc, &mut self.base.view);
                self.base.doc.buff[self.base.doc.cur.y].to_mut().clear();
                self.base.change_mode(Mode::Other(Write));
            }
            ChangeLeft => change!(self, left, REPEAT),
            ChangeRight => change!(self, right, REPEAT),
            ChangeNextWord => change!(self, next_word, REPEAT),
            ChangePrevWord => change!(self, prev_word, REPEAT),
            ChangeToBeginningOfLine => change!(self, beginning_of_line),
            ChangeToEndOfLine => change!(self, end_of_line),
            ChangeToMatchingOpposite => change!(self, matching_opposite),
            ChangeToEndOfFile => change!(self, end_of_file),
            ChangeToBeginningOfFile => change!(self, beginning_of_file),
            Paste => {
                let content = match self.base.clipboard.get_text() {
                    Ok(content) => content,
                    Err(err) => {
                        self.base.motion_repeat.clear();
                        return CommandResult::SetAndChangeBuffer(
                            INFO_BUFF_IDX,
                            vec![Cow::from(err.to_string())],
                            None,
                        );
                    }
                };

                self.base.doc.write_str(&content);
            }
            ReplaceChar(ch) => {
                if self.base.doc.buff[self.base.doc.cur.y]
                    .chars()
                    .nth(self.base.doc.cur.x)
                    .is_some()
                {
                    self.base.doc.delete_char();

                    match ch {
                        '\n' => edit::write_new_line_char(&mut self.base.doc, &mut self.base.view),
                        '\t' => edit::write_tab(&mut self.base.doc, &mut self.base.view),
                        _ => self.base.doc.write_char(ch),
                    }
                }
            }
        }

        // Rest motion repeat buffer after successful command.
        self.base.motion_repeat.clear();
        CommandResult::Ok
    }

    fn write_tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use WriteAction::*;

        match self.write_state_machine.tick(key.into()) {
            A(ViewMode) => self.base.change_mode(Mode::View),
            A(Left) => cursor::left(&mut self.base.doc, &mut self.base.view, 1),
            A(Down) => cursor::down(&mut self.base.doc, &mut self.base.view, 1),
            A(Up) => cursor::up(&mut self.base.doc, &mut self.base.view, 1),
            A(Right) => cursor::right(&mut self.base.doc, &mut self.base.view, 1),
            A(NextWord) => cursor::next_word(&mut self.base.doc, &mut self.base.view, 1),
            A(PrevWord) => cursor::prev_word(&mut self.base.doc, &mut self.base.view, 1),
            A(Newline) => edit::write_new_line_char(&mut self.base.doc, &mut self.base.view),
            A(Tab) => edit::write_tab(&mut self.base.doc, &mut self.base.view),
            A(DeleteChar) => edit::delete_char(&mut self.base.doc, &mut self.base.view),
            Invalid => {
                if let Some(Key::Char(ch)) = key {
                    edit::write_char(&mut self.base.doc, &mut self.base.view, ch);
                }
            }
            Incomplete => {}
        }

        CommandResult::Ok
    }

    fn command_tick(&mut self, tick: CommandTick<()>) -> CommandResult {
        use CommandTick::*;

        match tick {
            Apply => {
                let res = self.apply_command();
                self.base.change_mode(Mode::View);

                res
            }
            Other(()) => unreachable!("Illegal state"),
        }
    }
}

impl Buffer for TextBuffer {
    fn need_rerender(&self) -> bool {
        self.base.rerender
    }

    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        self.base.rerender = false;

        self.info_line().map_err(Error::other)?;

        let cursor_style = match self.base.mode {
            Mode::View => CursorStyle::BlinkingBlock,
            Mode::Command | Mode::Other(OtherMode::Write) => CursorStyle::BlinkingBar,
        };
        self.base.view.cmd = self.base.command_line();
        self.base
            .view
            .render(stdout, &self.base.doc, self.base.sel, cursor_style)
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.base.view.w == w && self.base.view.h == h {
            return;
        }

        self.base.rerender = true;

        self.base.view.resize(w, h, self.base.doc.buff.len());
    }

    fn tick(&mut self, key: Option<Key>) -> CommandResult {
        // Only rerender if input was received.
        self.base.rerender |= key.is_some();
        match self.base.mode {
            Mode::View => match self.base.view_tick(key) {
                Ok(res) => res,
                Err(action) => self.view_action(action),
            },
            Mode::Command => match self.base.command_tick(key) {
                Ok(res) => res,
                Err(tick) => self.command_tick(tick),
            },
            Mode::Other(OtherMode::Write) => self.write_tick(key),
        }
    }

    fn set_contents(&mut self, contents: &[Cow<'static, str>], path: Option<PathBuf>) {
        self.base.doc.set_contents(contents, 0, 0);
        self.base.view.cur = Cursor::new(0, 0);
        if let Some(path) = path {
            self.file = open_file(path).ok();
        }

        self.base.sel = None;
        self.base.change_mode(Mode::View);
        self.base.motion_repeat.clear();

        self.base.rerender = true;
    }

    fn can_quit(&self) -> Result<(), Vec<Cow<'static, str>>> {
        if !self.base.doc.edited {
            return Ok(());
        }

        Err(vec![Cow::from(
            "There are unsaved changes in the text buffer",
        )])
    }
}
