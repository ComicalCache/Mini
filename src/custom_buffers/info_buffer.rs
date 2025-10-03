use crate::{
    FILES_BUFF_IDX, TXT_BUFF_IDX,
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
    ChangeToFilesBuffer,
    Repeat(char),
}

pub struct InfoBuffer {
    doc: Document,
    view: Viewport,

    sel: Option<Cursor>,
    motion_repeat: String,
    clipboard: Clipboard,

    input_state_machine: StateMachine<Action>,

    rerender: bool,
}

impl InfoBuffer {
    pub fn new(w: usize, h: usize) -> Result<Self, Error> {
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
            .simple(Key::Char('e'), ChangeToFilesBuffer)
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
        let input_state_machine = StateMachine::new(command_map, Duration::from_secs(1));

        Ok(InfoBuffer {
            doc: Document::new(0, 0, None),
            view: Viewport::new(w, h, 0, 0, 1),
            sel: None,
            motion_repeat: String::new(),
            clipboard: Clipboard::new().map_err(Error::other)?,
            input_state_machine,
            rerender: false,
        })
    }

    fn info_line(&mut self) -> Result<(), std::fmt::Error> {
        use std::fmt::Write;

        self.view.info_line.clear();

        // Plus 1 since text coordinates are 0 indexed.
        let line = self.doc.cur.y + 1;
        let col = self.doc.cur.x + 1;
        let total = self.doc.buff.len();
        let percentage = 100 * line / total;

        write!(
            &mut self.view.info_line,
            "[Info] [{line}:{col}/{total} {percentage}%]",
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

        let edited = if self.doc.edited { '*' } else { ' ' };
        write!(&mut self.view.info_line, " {edited}")
    }
}

impl Buffer for InfoBuffer {
    fn need_rerender(&self) -> bool {
        self.rerender
    }

    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        self.rerender = false;

        self.info_line().map_err(Error::other)?;
        self.view.cmd = None;
        self.view
            .render(stdout, &self.doc, self.sel, CursorStyle::BlinkingBlock)
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.w == w && self.view.h == h {
            return;
        }

        self.rerender = true;

        self.view.resize(w, h, self.doc.buff.len());
    }

    fn tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};
        #[allow(clippy::enum_glob_use)]
        use Action::*;

        // Only rerender if input was received.
        self.rerender |= key.is_some();
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
            A(ChangeToFilesBuffer) => return CommandResult::ChangeBuffer(FILES_BUFF_IDX),
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

    fn set_contents(&mut self, contents: &[Cow<'static, str>], _: Option<PathBuf>) {
        self.doc.set_contents(contents, 0, 0);
        self.view.cur = Cursor::new(0, 0);

        self.sel = None;
        self.motion_repeat.clear();

        self.rerender = true;
    }

    fn can_quit(&self) -> Result<(), Vec<Cow<'static, str>>> {
        Ok(())
    }
}
