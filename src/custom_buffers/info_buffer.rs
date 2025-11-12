mod apply_command;

use crate::{
    FILES_BUFF_IDX, TXT_BUFF_IDX,
    buffer::{
        Buffer,
        base::{BaseBuffer, COMMAND_PROMPT, CommandTick, Mode, ViewAction},
    },
    c_buff,
    cursor::Cursor,
    display::Display,
    util::{CommandResult, CursorStyle},
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

        Ok(Self { base })
    }

    /// Creates an info line
    fn info_line(&mut self) {
        use std::fmt::Write;

        self.base.info.buff[0].to_mut().clear();

        let mode = match self.base.mode {
            Mode::View => "V",
            Mode::Command => "C",
            Mode::Other(()) => unreachable!(),
        };
        // Plus 1 since text coordinates are 0 indexed.
        let line = self.base.doc.cur.y + 1;
        let col = self.base.doc.cur.x + 1;
        let total = self.base.doc.buff.len();
        let percentage = 100 * line / total;

        write!(
            self.base.info.buff[0].to_mut(),
            "[Info] [{mode}] [{line}:{col}/{total} {percentage}%]",
        )
        .unwrap();

        if let Some(pos) = self.base.sel {
            // Plus 1 since text coordinates are 0 indexed.
            let line = pos.y + 1;
            let col = pos.x + 1;
            write!(
                self.base.info.buff[0].to_mut(),
                " [Selected {line}:{col} - {}:{}]",
                self.base.doc.cur.y + 1,
                self.base.doc.cur.x + 1
            )
            .unwrap();
        }
    }

    /// Handles self defined view actions.
    fn view_action(&mut self, action: OtherViewAction) -> CommandResult {
        use OtherViewAction::*;

        match action {
            ChangeToTextBuffer => c_buff!(self, TXT_BUFF_IDX),
            ChangeToFilesBuffer => c_buff!(self, FILES_BUFF_IDX),
        }

        // Rest motion repeat buffer after successful command.
        // self.base.motion_repeat.clear();
        // CommandResult::Ok
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
            // Update the contiguous buffer only if the document has been edited.
            if self.base.doc.edited {
                // FIXME: use a diffing approach to only replace/move whats necessary.
                self.base.doc.contiguous_buff.clear();
                for line in &self.base.doc.buff {
                    self.base.doc.contiguous_buff.push_str(line);
                    self.base.doc.contiguous_buff.push('\n');
                }
                // Remove the last trailing newline.
                self.base.doc.contiguous_buff.pop().unwrap();
            }

            let contents = &self.base.doc.contiguous_buff;
            self.base.doc.highlighter.highlight(contents);
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
                &self.base.cmd.buff[0],
                0,
                display,
                &self.base.cmd,
                COMMAND_PROMPT,
            );
        } else {
            self.base.info_view.render_bar(
                &self.base.info.buff[0],
                0,
                display,
                &self.base.info,
                "",
            );
        }

        self.base.doc_view.render_gutter(display, &self.base.doc);
        self.base
            .doc_view
            .render_document(display, &self.base.doc, self.base.sel);

        let (view, off) = if cmd {
            (&self.base.cmd_view, Some(COMMAND_PROMPT.chars().count()))
        } else {
            (&self.base.doc_view, None)
        };
        view.render_cursor(display, cursor_style, off);
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
            Mode::Other(()) => unreachable!(),
        }
    }

    fn set_contents(&mut self, contents: String, _: Option<PathBuf>, _: Option<String>) {
        // Set contents moves the doc.cur to the beginning.
        self.base.doc.set_contents(contents);
        self.base.doc_view.cur = Cursor::new(0, 0);

        self.base.sel = None;
        self.base.motion_repeat.clear();
        self.base.clear_matches();

        self.base.rerender = true;
    }

    fn can_quit(&self) -> Result<(), String> {
        Ok(())
    }
}
