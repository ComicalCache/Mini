mod r#move;

use crate::{
    FILES_BUFF_IDX, TXT_BUFF_IDX,
    buffer::Buffer,
    document::Document,
    state_machine::{CommandMap, StateMachine},
    util::{CommandResult, CursorStyle},
    viewport::Viewport,
};
use std::{
    io::{BufWriter, Error, Stdout},
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
        let command_map = CommandMap::new()
            .simple(Key::Char('h'), Action::Left)
            .simple(Key::Char('j'), Action::Down)
            .simple(Key::Char('k'), Action::Up)
            .simple(Key::Char('l'), Action::Right)
            .simple(Key::Char('w'), Action::NextWord)
            .simple(Key::Char('b'), Action::PrevWord)
            .simple(Key::Char('<'), Action::JumpToBeginningOfLine)
            .simple(Key::Char('>'), Action::JumpToEndOfLine)
            .simple(Key::Char('.'), Action::JumpToMatchingOpposite)
            .simple(Key::Char('g'), Action::JumpToEndOfFile)
            .simple(Key::Char('G'), Action::JumpToBeginningOfFile)
            .simple(Key::Char('t'), Action::ChangeToTextBuffer)
            .simple(Key::Char('e'), Action::ChangeToFilesBuffer);
        let input_state_machine = StateMachine::new(command_map, Duration::from_secs(1));

        InfoBuffer {
            doc: Document::new(None, 0, 0),
            view: Viewport::new(w, h, 0, h / 2),
            input_state_machine,
        }
    }

    fn info_line(&self) -> String {
        use std::fmt::Write;

        let mut info_line = String::new();

        // Plus 1 since text coordinates are 0 indexed.
        let line = self.doc.cursor.y + 1;
        let col = self.doc.cursor.x + 1;
        let total = self.doc.lines.len();
        let percentage = 100 * line / total;

        write!(
            &mut info_line,
            "[Info] [{line}:{col}/{total} {percentage}%]",
        )
        .unwrap();

        let edited = if self.doc.edited { '*' } else { ' ' };
        write!(&mut info_line, " {edited}").unwrap();

        info_line
    }
}

impl Buffer for InfoBuffer {
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        self.view.render(
            stdout,
            &self.doc,
            self.info_line(),
            None,
            CursorStyle::BlinkingBlock,
        )
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.w == w && self.view.h == h {
            return;
        }

        self.view.resize(w, h, self.view.cursor.x.min(w), h / 2);
    }

    fn tick(&mut self, key: Option<Key>) -> CommandResult {
        use crate::state_machine::StateMachineResult::{Action as A, Incomplete, Invalid};

        match self.input_state_machine.tick(key.into()) {
            A(Action::Left) => self.left(1),
            A(Action::Down) => self.down(1),
            A(Action::Up) => self.up(1),
            A(Action::Right) => self.right(1),
            A(Action::NextWord) => self.next_word(),
            A(Action::PrevWord) => self.prev_word(),
            A(Action::JumpToBeginningOfLine) => self.jump_to_beginning_of_line(),
            A(Action::JumpToEndOfLine) => self.jump_to_end_of_line(),
            A(Action::JumpToMatchingOpposite) => self.jump_to_matching_opposite(),
            A(Action::JumpToEndOfFile) => self.jump_to_end_of_file(),
            A(Action::JumpToBeginningOfFile) => self.jump_to_beginning_of_file(),
            A(Action::ChangeToTextBuffer) => return CommandResult::ChangeBuffer(TXT_BUFF_IDX),
            A(Action::ChangeToFilesBuffer) => return CommandResult::ChangeBuffer(FILES_BUFF_IDX),
            Incomplete | Invalid => {}
        }

        CommandResult::Ok
    }

    fn set_contents(&mut self, contents: &[String]) {
        self.doc.set_contents(contents, 0, 0);
    }

    fn can_quit(&self) -> Result<(), Vec<String>> {
        Ok(())
    }
}
