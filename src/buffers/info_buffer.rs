use crate::{
    FILES_BUFF_IDX, TXT_BUFF_IDX,
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
    ChangeToTextBuffer,
    ChangeToFilesBuffer,
}

pub struct InfoBuffer {
    doc: Document,
    view: Viewport,
    input_state_machine: StateMachine<Action>,
}

impl InfoBuffer {
    pub fn new(w: usize, h: usize) -> Self {
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
            .simple(Key::Char('t'), ChangeToTextBuffer)
            .simple(Key::Char('e'), ChangeToFilesBuffer);
        let input_state_machine = StateMachine::new(command_map, Duration::from_secs(1));

        InfoBuffer {
            doc: Document::new(0, 0, None),
            view: Viewport::new(w, h, 0, h / 2),
            input_state_machine,
        }
    }

    fn info_line(&mut self) {
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
        )
        .unwrap();

        let edited = if self.doc.edited { '*' } else { ' ' };
        write!(&mut self.view.info_line, " {edited}").unwrap();
    }
}

impl Buffer for InfoBuffer {
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        self.info_line();
        self.view.cmd = None;
        self.view
            .render(stdout, &self.doc, CursorStyle::BlinkingBlock)
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
            A(ChangeToTextBuffer) => return CommandResult::ChangeBuffer(TXT_BUFF_IDX),
            A(ChangeToFilesBuffer) => return CommandResult::ChangeBuffer(FILES_BUFF_IDX),
            Incomplete | Invalid => {}
        }

        CommandResult::Ok
    }

    fn set_contents(&mut self, contents: &[Cow<'static, str>], _: Option<PathBuf>) {
        self.doc.set_contents(contents, 0, 0);
    }

    fn can_quit(&self) -> Result<(), Vec<Cow<'static, str>>> {
        Ok(())
    }
}
