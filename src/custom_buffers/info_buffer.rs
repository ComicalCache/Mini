mod apply_command;

use crate::{
    FILES_BUFF_IDX, TXT_BUFF_IDX,
    buffer::{
        Buffer,
        base::{BaseBuffer, CommandTick, Mode, ViewAction},
    },
    change_buffer,
    cursor::Cursor,
    util::{CommandResult, CursorStyle},
};
use std::{
    borrow::Cow,
    io::{BufWriter, Error, Stdout},
    path::PathBuf,
};
use termion::{event::Key, raw::RawTerminal};

#[derive(Clone, Copy)]
enum OtherViewAction {
    // Buffers
    ChangeToTextBuffer,
    ChangeToFilesBuffer,
}

pub struct InfoBuffer {
    base: BaseBuffer<(), OtherViewAction, ()>,
}

impl InfoBuffer {
    pub fn new(w: usize, h: usize) -> Result<Self, Error> {
        use OtherViewAction::*;

        let mut base = BaseBuffer::new(w, h, 1, None)?;
        base.view_state_machine.command_map = base
            .view_state_machine
            .command_map
            .simple(Key::Char('t'), ViewAction::Other(ChangeToTextBuffer))
            .simple(Key::Char('e'), ViewAction::Other(ChangeToFilesBuffer));

        Ok(InfoBuffer { base })
    }

    fn info_line(&mut self) -> Result<(), std::fmt::Error> {
        use std::fmt::Write;

        self.base.view.info_line.clear();

        let mode = match self.base.mode {
            Mode::View => "V",
            Mode::Command => "C",
            Mode::Other(()) => unreachable!("Illegal state"),
        };
        // Plus 1 since text coordinates are 0 indexed.
        let line = self.base.doc.cur.y + 1;
        let col = self.base.doc.cur.x + 1;
        let total = self.base.doc.buff.len();
        let percentage = 100 * line / total;

        write!(
            &mut self.base.view.info_line,
            "[Info] [{mode}] [{line}:{col}/{total} {percentage}%]",
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
        write!(&mut self.base.view.info_line, " {edited}")
    }

    fn view_action(&mut self, action: OtherViewAction) -> CommandResult {
        use OtherViewAction::*;

        match action {
            ChangeToTextBuffer => change_buffer!(self, TXT_BUFF_IDX),
            ChangeToFilesBuffer => change_buffer!(self, FILES_BUFF_IDX),
        }

        // Rest motion repeat buffer after successful command.
        // self.base.motion_repeat.clear();
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

impl Buffer for InfoBuffer {
    fn need_rerender(&self) -> bool {
        self.base.rerender
    }

    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        self.base.rerender = false;

        self.info_line().map_err(Error::other)?;

        let cursor_style = match self.base.mode {
            Mode::Command => CursorStyle::BlinkingBar,
            Mode::View | Mode::Other(()) => CursorStyle::BlinkingBlock,
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
            Mode::Other(()) => unreachable!("Illegal state"),
        }
    }

    fn set_contents(&mut self, contents: &[Cow<'static, str>], _: Option<PathBuf>) {
        self.base.doc.set_contents(contents, 0, 0);
        self.base.view.cur = Cursor::new(0, 0);

        self.base.sel = None;
        self.base.motion_repeat.clear();

        self.base.rerender = true;
    }

    fn can_quit(&self) -> Result<(), Vec<Cow<'static, str>>> {
        Ok(())
    }
}
