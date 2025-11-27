mod apply_command;
mod history;
mod insert;

use crate::{
    FILES_BUFF_IDX, INFO_BUFF_IDX, TXT_BUFF_IDX,
    buffer::{
        Buffer,
        base::{BaseBuffer, CommandTick, Mode, ViewAction},
        delete, edit,
    },
    cursor::{self, Cursor, CursorStyle},
    display::Display,
    document::Document,
    history::History,
    state_machine::{ChainResult, CommandMap, StateMachine},
    util::{CommandResult, open_file},
};
use std::{
    fs::File,
    io::{Error, Read},
    path::PathBuf,
    time::Duration,
};
use termion::event::Key;

macro_rules! delete {
    ($self:ident, $func:ident) => {{
        delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            Some(&mut $self.history),
        );
        $self.base.clear_matches();
    }};
    ($self:ident, $func:ident, REPEAT) => {{
        delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            Some(&mut $self.history),
            1,
        );
        $self.base.clear_matches();
    }};
    ($self:ident, $func:ident, SELECTION) => {{
        delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            &mut $self.base.sel,
            Some(&mut $self.history),
        );
        $self.base.clear_matches();
    }};
}

macro_rules! change {
    ($self:ident, $func:ident) => {{
        delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            Some(&mut $self.history),
        );
        $self.base.change_mode(Mode::Other(Write));
    }};
    ($self:ident, $func:ident, REPEAT) => {{
        delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            Some(&mut $self.history),
            1,
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
    ChangeToTextBuffer,
    ChangeToInfoBuffer,
    ChangeToFilesBuffer,

    // Delete
    DeleteChar,
    DeleteSelection,
    DeleteLine,
    DeleteLeft,
    DeleteRight,
    DeleteNextWord,
    DeletePrevWord,
    DeleteNextWordEnd,
    DeletePrevWordEnd,
    DeleteNextWhitespace,
    DeletePrevWhitespace,
    DeleteNextEmptyLine,
    DeletePrevEmptyLine,
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
    ChangeNextWordEnd,
    ChangePrevWordEnd,
    ChangeNextWhitespace,
    ChangePrevWhitespace,
    ChangeNextEmptyLine,
    ChangePrevEmptyLine,
    ChangeToBeginningOfLine,
    ChangeToEndOfLine,
    ChangeToMatchingOpposite,
    ChangeToBeginningOfFile,
    ChangeToEndOfFile,
    ReplaceChar(char),

    // Paste
    Paste,
    PasteAbove,

    // History
    Undo,
    Redo,
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
    Tab,
    DeleteChar,
}

#[derive(Clone, Copy)]
enum OtherMode {
    Write,
}

/// A text buffer.
pub struct TextBuffer {
    base: BaseBuffer<OtherMode, OtherViewAction, ()>,

    /// The info bar content.
    info: Document,

    /// The opened file.
    file: Option<File>,
    /// The name of the opened file.
    file_name: Option<String>,

    /// A history of edits to undo and redo.
    history: History,

    /// The state machine handling input in write mode.
    write_state_machine: StateMachine<WriteAction>,
}

impl TextBuffer {
    pub fn new(
        w: usize,
        h: usize,
        x_off: usize,
        y_off: usize,
        mut file: Option<File>,
        file_name: Option<String>,
    ) -> Result<Self, Error> {
        let contents = if let Some(file) = file.as_mut() {
            let mut buff = String::new();
            file.read_to_string(&mut buff)?;

            Some(buff)
        } else {
            None
        };

        let mut base = BaseBuffer::new(w, h, x_off, y_off, contents)?;
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
                .simple(Key::Char('t'), Other(ChangeToTextBuffer))
                .simple(Key::Char('?'), Other(ChangeToInfoBuffer))
                .simple(Key::Char('e'), Other(ChangeToFilesBuffer))
                .operator(Key::Char('d'), |key| match key {
                    Key::Char('v') => Some(ChainResult::Action(Other(DeleteSelection))),
                    Key::Char('d') => Some(ChainResult::Action(Other(DeleteLine))),
                    Key::Char('h') => Some(ChainResult::Action(Other(DeleteLeft))),
                    Key::Char('l') => Some(ChainResult::Action(Other(DeleteRight))),
                    Key::Char('w') => Some(ChainResult::Action(Other(DeleteNextWord))),
                    Key::Char('W') => Some(ChainResult::Action(Other(DeleteNextWordEnd))),
                    Key::Char('b') => Some(ChainResult::Action(Other(DeletePrevWord))),
                    Key::Char('B') => Some(ChainResult::Action(Other(DeletePrevWordEnd))),
                    Key::Char('s') => Some(ChainResult::Action(Other(DeleteNextWhitespace))),
                    Key::Char('S') => Some(ChainResult::Action(Other(DeletePrevWhitespace))),
                    Key::Char('}') => Some(ChainResult::Action(Other(DeleteNextEmptyLine))),
                    Key::Char('{') => Some(ChainResult::Action(Other(DeletePrevEmptyLine))),
                    Key::Char('<') => Some(ChainResult::Action(Other(DeleteToBeginningOfLine))),
                    Key::Char('>') => Some(ChainResult::Action(Other(DeleteToEndOfLine))),
                    Key::Char('.') => Some(ChainResult::Action(Other(DeleteToMatchingOpposite))),
                    Key::Char('g') => Some(ChainResult::Action(Other(DeleteToEndOfFile))),
                    Key::Char('G') => Some(ChainResult::Action(Other(DeleteToBeginningOfFile))),
                    _ => None,
                })
                .simple(Key::Char('x'), Other(DeleteChar))
                .operator(Key::Char('c'), |key| match key {
                    Key::Char('v') => Some(ChainResult::Action(Other(ChangeSelection))),
                    Key::Char('c') => Some(ChainResult::Action(Other(ChangeLine))),
                    Key::Char('h') => Some(ChainResult::Action(Other(ChangeLeft))),
                    Key::Char('l') => Some(ChainResult::Action(Other(ChangeRight))),
                    Key::Char('w') => Some(ChainResult::Action(Other(ChangeNextWord))),
                    Key::Char('W') => Some(ChainResult::Action(Other(ChangeNextWordEnd))),
                    Key::Char('b') => Some(ChainResult::Action(Other(ChangePrevWord))),
                    Key::Char('B') => Some(ChainResult::Action(Other(ChangePrevWordEnd))),
                    Key::Char('s') => Some(ChainResult::Action(Other(ChangeNextWhitespace))),
                    Key::Char('S') => Some(ChainResult::Action(Other(ChangePrevWhitespace))),
                    Key::Char('}') => Some(ChainResult::Action(Other(ChangeNextEmptyLine))),
                    Key::Char('{') => Some(ChainResult::Action(Other(ChangePrevEmptyLine))),
                    Key::Char('<') => Some(ChainResult::Action(Other(ChangeToBeginningOfLine))),
                    Key::Char('>') => Some(ChainResult::Action(Other(ChangeToEndOfLine))),
                    Key::Char('.') => Some(ChainResult::Action(Other(ChangeToMatchingOpposite))),
                    Key::Char('g') => Some(ChainResult::Action(Other(ChangeToEndOfFile))),
                    Key::Char('G') => Some(ChainResult::Action(Other(ChangeToBeginningOfFile))),
                    _ => None,
                })
                .simple(Key::Char('p'), Other(Paste))
                .simple(Key::Char('P'), Other(PasteAbove))
                .prefix(Key::Char('r'), |key| match key {
                    Key::Char(ch) => Some(ChainResult::Action(Other(ReplaceChar(ch)))),
                    _ => None,
                })
                .simple(Key::Char('u'), Other(Undo))
                .simple(Key::Char('U'), Other(Redo));
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
                .simple(Key::Char('\t'), Tab)
                .simple(Key::Backspace, DeleteChar);
            StateMachine::new(command_map, Duration::from_secs(1))
        };

        Ok(Self {
            base,
            info: Document::new(0, 0, None),
            file,
            file_name,
            history: History::new(),
            write_state_machine,
        })
    }

    /// Creates an info line
    fn info_line(&mut self) {
        use std::fmt::Write;

        let mut info_line = String::new();

        let mode = match self.base.mode {
            Mode::View => "V",
            Mode::Command => "C",
            Mode::Other(OtherMode::Write) => "W",
        };
        // Plus 1 since text coordinates are 0 indexed.
        let line = self.base.doc.cur.y + 1;
        let col = self.base.doc.cur.x + 1;
        let total = self.base.doc.len();
        let percentage = 100 * line / total;
        let size: usize = self.base.doc.lines().map(|l| l.bytes().len()).sum();

        let indicator = match &self.file {
            Some(_) => self.file_name.as_ref().unwrap(),
            None => "Text",
        };
        write!(
            &mut info_line,
            "[{indicator}] [{mode}] [{line}:{col}] [{line}/{total} {percentage}%] [{size}B]",
        )
        .unwrap();

        if let Some(pos) = self.base.sel {
            let (start, end) = if pos < self.base.doc.cur {
                (pos, self.base.doc.cur)
            } else {
                (self.base.doc.cur, pos)
            };

            // Plus 1 since text coordinates are 0 indexed.
            write!(
                &mut info_line,
                " [Selected {}:{} - {}:{}]",
                start.y + 1,
                start.x + 1,
                end.y + 1,
                end.x + 1
            )
            .unwrap();
        }

        let edited = if self.base.doc.edited { '*' } else { ' ' };
        write!(&mut info_line, " {edited}").unwrap();

        self.info.from(info_line.as_str());
    }

    /// Handles self defined view actions.
    fn view_action(&mut self, action: OtherViewAction) -> CommandResult {
        use OtherMode::Write;
        use OtherViewAction::*;

        match action {
            Insert => self.base.change_mode(Mode::Other(Write)),
            Append => {
                cursor::right(&mut self.base.doc, &mut self.base.doc_view, 1);
                self.base.change_mode(Mode::Other(Write));
            }
            AppendEndOfLine => {
                cursor::jump_to_end_of_line(&mut self.base.doc, &mut self.base.doc_view);
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
            ChangeToTextBuffer => return CommandResult::Change(TXT_BUFF_IDX),
            ChangeToInfoBuffer => return CommandResult::Change(INFO_BUFF_IDX),
            ChangeToFilesBuffer => return CommandResult::Change(FILES_BUFF_IDX),
            DeleteChar | DeleteRight => delete!(self, right, REPEAT),
            DeleteSelection => delete!(self, selection, SELECTION),
            DeleteLine => delete!(self, line, REPEAT),
            DeleteLeft => delete!(self, left, REPEAT),
            DeleteNextWord => delete!(self, next_word, REPEAT),
            DeletePrevWord => delete!(self, prev_word, REPEAT),
            DeleteNextWordEnd => delete!(self, next_word_end, REPEAT),
            DeletePrevWordEnd => delete!(self, prev_word_end, REPEAT),
            DeleteNextWhitespace => delete!(self, next_whitespace, REPEAT),
            DeletePrevWhitespace => delete!(self, prev_whitespace, REPEAT),
            DeleteNextEmptyLine => delete!(self, next_empty_line, REPEAT),
            DeletePrevEmptyLine => delete!(self, prev_empty_line, REPEAT),
            DeleteToBeginningOfLine => delete!(self, beginning_of_line),
            DeleteToEndOfLine => delete!(self, end_of_line),
            DeleteToMatchingOpposite => delete!(self, matching_opposite),
            DeleteToEndOfFile => delete!(self, end_of_file),
            DeleteToBeginningOfFile => delete!(self, beginning_of_file),
            ChangeSelection => {
                delete::selection(
                    &mut self.base.doc,
                    &mut self.base.doc_view,
                    &mut self.base.sel,
                    Some(&mut self.history),
                );
                self.base.change_mode(Mode::Other(Write));
            }
            ChangeLine => {
                cursor::jump_to_beginning_of_line(&mut self.base.doc, &mut self.base.doc_view);
                delete::end_of_line(
                    &mut self.base.doc,
                    &mut self.base.doc_view,
                    Some(&mut self.history),
                );
                self.base.change_mode(Mode::Other(Write));
            }
            ChangeLeft => change!(self, left, REPEAT),
            ChangeRight => change!(self, right, REPEAT),
            ChangeNextWord => change!(self, next_word, REPEAT),
            ChangePrevWord => change!(self, prev_word, REPEAT),
            ChangeNextWordEnd => change!(self, next_word_end, REPEAT),
            ChangePrevWordEnd => change!(self, prev_word_end, REPEAT),
            ChangeNextWhitespace => change!(self, next_whitespace, REPEAT),
            ChangePrevWhitespace => change!(self, prev_whitespace, REPEAT),
            ChangeNextEmptyLine => change!(self, next_empty_line, REPEAT),
            ChangePrevEmptyLine => change!(self, prev_empty_line, REPEAT),
            ChangeToBeginningOfLine => change!(self, beginning_of_line),
            ChangeToEndOfLine => change!(self, end_of_line),
            ChangeToMatchingOpposite => change!(self, matching_opposite),
            ChangeToEndOfFile => change!(self, end_of_file),
            ChangeToBeginningOfFile => change!(self, beginning_of_file),
            Paste => {
                if let Some(res) = self.paste(false, false) {
                    return res;
                }
                self.base.clear_matches();
            }
            PasteAbove => {
                self.insert_move_new_line_above();
                if let Some(res) = self.paste(true, false) {
                    return res;
                }
                self.base.clear_matches();
            }
            ReplaceChar(ch) => {
                self.replace(ch);
                self.base.clear_matches();
            }
            Undo => self.undo(),
            Redo => self.redo(),
        }

        CommandResult::Ok
    }

    /// Handles write mode ticks.
    fn write_tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use WriteAction::*;

        match self.write_state_machine.tick(key.into()) {
            A(ViewMode) => self.base.change_mode(Mode::View),
            A(Left) => cursor::left(&mut self.base.doc, &mut self.base.doc_view, 1),
            A(Down) => cursor::down(&mut self.base.doc, &mut self.base.doc_view, 1),
            A(Up) => cursor::up(&mut self.base.doc, &mut self.base.doc_view, 1),
            A(Right) => cursor::right(&mut self.base.doc, &mut self.base.doc_view, 1),
            A(NextWord) => cursor::next_word(&mut self.base.doc, &mut self.base.doc_view, 1),
            A(PrevWord) => cursor::prev_word(&mut self.base.doc, &mut self.base.doc_view, 1),
            A(Tab) => edit::write_tab(
                &mut self.base.doc,
                &mut self.base.doc_view,
                Some(&mut self.history),
                true,
            ),
            A(DeleteChar) => edit::delete_char(
                &mut self.base.doc,
                &mut self.base.doc_view,
                Some(&mut self.history),
            ),
            Invalid => {
                if let Some(Key::Char(ch)) = key {
                    edit::write_char(
                        &mut self.base.doc,
                        &mut self.base.doc_view,
                        Some(&mut self.history),
                        ch,
                    );
                }
            }
            Incomplete => {}
        }

        CommandResult::Ok
    }

    /// Handles self apply and self defined command ticks.
    fn command_tick(&mut self, tick: CommandTick<()>) -> CommandResult {
        use CommandTick::*;

        match tick {
            Apply(cmd) => self.apply_command(&cmd),
            Other(()) => unreachable!(),
        }
    }
}

impl Buffer for TextBuffer {
    fn need_rerender(&self) -> bool {
        self.base.rerender
    }

    fn render(&mut self, display: &mut Display) {
        self.base.rerender = false;

        self.info_line();

        let (cursor_style, cmd) = match self.base.mode {
            Mode::View => (CursorStyle::BlinkingBlock, false),
            Mode::Command => (CursorStyle::BlinkingBar, true),
            Mode::Other(OtherMode::Write) => (CursorStyle::BlinkingBar, false),
        };

        if cmd {
            self.base.cmd_view.render_bar(
                self.base.cmd.line(0).unwrap().to_string().as_str(),
                0,
                display,
                &self.base.cmd,
            );
        } else {
            self.base.info_view.render_bar(
                self.info.line(0).unwrap().to_string().as_str(),
                0,
                display,
                &self.info,
            );
        }

        self.base.doc_view.render_gutter(display, &self.base.doc);
        self.base
            .doc_view
            .render_document(display, &self.base.doc, self.base.sel);

        let view = if cmd {
            &self.base.cmd_view
        } else {
            &self.base.doc_view
        };
        view.render_cursor(display, cursor_style);
    }

    fn resize(&mut self, w: usize, h: usize, x_off: usize, y_off: usize) {
        self.base.rerender = true;
        self.base.resize(w, h, x_off, y_off);
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

    fn set_contents(&mut self, contents: String, path: Option<PathBuf>, file_name: Option<String>) {
        // Set contents moves the doc.cur to the beginning.
        self.base.doc.from(contents.as_str());
        self.base.doc_view.cur = Cursor::new(0, 0);
        if let Some(path) = path {
            self.file = open_file(path).ok();
            self.file_name = file_name;
        }

        self.base.sel = None;
        self.base.change_mode(Mode::View);
        self.base.clear_matches();

        self.history.clear();

        self.base.rerender = true;
    }

    fn can_quit(&self) -> Result<(), String> {
        if !self.base.doc.edited {
            return Ok(());
        }

        Err("There are unsaved changes in the text buffer".to_string())
    }
}
