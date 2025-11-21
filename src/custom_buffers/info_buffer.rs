mod apply_command;

use crate::{
    FILES_BUFF_IDX, TXT_BUFF_IDX,
    buffer::{
        Buffer,
        base::{BaseBuffer, CommandTick, Mode, ViewAction},
    },
    cursor::{Cursor, CursorStyle},
    display::Display,
    document::Document,
    util::CommandResult,
};
use std::{io::Error, path::PathBuf};
use termion::event::Key;

#[derive(Clone, Copy)]
enum OtherViewAction {
    // Buffers
    ChangeToTextBuffer,
    ChangeToFilesBuffer,
}

/// A buffer to show read only information.
pub struct InfoBuffer {
    base: BaseBuffer<(), OtherViewAction, ()>,

    /// The info bar content.
    info: Document,
}

impl InfoBuffer {
    pub fn new(w: usize, h: usize, x_off: usize, y_off: usize) -> Result<Self, Error> {
        use OtherViewAction::*;

        let mut base = BaseBuffer::new(w, h, x_off, y_off, None)?;
        base.view_state_machine.command_map = base
            .view_state_machine
            .command_map
            .simple(Key::Char('t'), ViewAction::Other(ChangeToTextBuffer))
            .simple(Key::Char('e'), ViewAction::Other(ChangeToFilesBuffer));

        Ok(Self {
            base,
            info: Document::new(0, 0, None),
        })
    }

    /// Creates an info line
    fn info_line(&mut self) {
        use std::fmt::Write;

        let mut info_line = String::new();

        let mode = match self.base.mode {
            Mode::View => "V",
            Mode::Command => "C",
            Mode::Other(()) => unreachable!(),
        };
        // Plus 1 since text coordinates are 0 indexed.
        let line = self.base.doc.cur.y + 1;
        let col = self.base.doc.cur.x + 1;
        let total = self.base.doc.len();
        let percentage = 100 * line / total;

        write!(
            &mut info_line,
            "[Info] [{mode}] [{line}:{col}] [{line}/{total} {percentage}%]",
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

        self.info.from(info_line.as_str());
    }

    /// Handles self defined view actions.
    const fn view_action(action: OtherViewAction) -> CommandResult {
        use OtherViewAction::*;

        match action {
            ChangeToTextBuffer => CommandResult::Change(TXT_BUFF_IDX),
            ChangeToFilesBuffer => CommandResult::Change(FILES_BUFF_IDX),
        }
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

impl Buffer for InfoBuffer {
    fn need_rerender(&self) -> bool {
        self.base.rerender
    }

    fn highlight(&mut self) {
        if self.need_rerender() {
            self.base.doc.highlight();
        }
    }

    fn render(&mut self, display: &mut Display) {
        self.base.rerender = false;

        self.info_line();

        let (cursor_style, cmd) = match self.base.mode {
            Mode::View | Mode::Other(()) => (CursorStyle::BlinkingBlock, false),
            Mode::Command => (CursorStyle::BlinkingBar, true),
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
                Err(action) => Self::view_action(action),
            },
            Mode::Command => match self.base.command_tick(key) {
                Ok(res) => res,
                Err(tick) => self.command_tick(tick),
            },
            Mode::Other(()) => unreachable!(),
        }
    }

    fn set_contents(&mut self, contents: String, _: Option<PathBuf>, _: Option<String>) {
        // Set contents moves the doc.cur to the beginning.
        self.base.doc.from(contents.as_str());
        self.base.doc_view.cur = Cursor::new(0, 0);

        self.base.sel = None;
        self.base.clear_matches();

        self.base.rerender = true;
    }

    fn can_quit(&self) -> Result<(), String> {
        Ok(())
    }
}
