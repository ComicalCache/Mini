mod apply_command;
mod interact;

use crate::{
    INFO_BUFF_IDX, TXT_BUFF_IDX,
    buffer::{
        Buffer,
        base::{BaseBuffer, COMMAND_PROMPT, CommandTick, Mode, ViewAction},
    },
    change_buffer,
    cursor::Cursor,
    util::{CommandResult, CursorStyle},
};
use std::{
    borrow::Cow,
    io::{BufWriter, Error, Stdout, Write},
    path::PathBuf,
};
use termion::{clear::All, cursor::Hide, event::Key, raw::RawTerminal};

#[derive(Clone, Copy)]
enum OtherViewAction {
    // Interact
    Refresh,
    SelectItem,

    // Buffers
    ChangeToTextBuffer,
    ChangeToInfoBuffer,
}

/// A file browser buffer.
pub struct FilesBuffer {
    base: BaseBuffer<(), OtherViewAction, ()>,
    /// The path of the current item.
    path: PathBuf,
    /// All entries of the dir containing the current item.
    entries: Vec<PathBuf>,
}

impl FilesBuffer {
    pub fn new(w: usize, h: usize, path: PathBuf) -> Result<Self, Error> {
        use OtherViewAction::*;

        let mut entries = Vec::new();
        let contents = FilesBuffer::load_dir(&path, &mut entries)?;

        let mut base = BaseBuffer::new(w, h, Some(contents))?;
        base.view_state_machine.command_map = base
            .view_state_machine
            .command_map
            .simple(Key::Char('t'), ViewAction::Other(ChangeToTextBuffer))
            .simple(Key::Char('?'), ViewAction::Other(ChangeToInfoBuffer))
            .simple(Key::Char('r'), ViewAction::Other(Refresh))
            .simple(Key::Char('\n'), ViewAction::Other(SelectItem));

        Ok(FilesBuffer {
            base,
            path,
            entries,
        })
    }

    /// Creates an info line
    fn info_line(&mut self) -> Result<(), std::fmt::Error> {
        use std::fmt::Write;

        self.base.info.buff[0].to_mut().clear();

        let mode = match self.base.mode {
            Mode::View => "V",
            Mode::Command => "C",
            Mode::Other(()) => unreachable!("Illegal state"),
        };
        // No plus 1 since the first entry is always ".." and not really a directory entry.
        let curr = self.base.doc.cur.y;
        let curr_type = match curr {
            0 => "Parent Dir",
            idx if self.entries[idx - 1].is_symlink() => "Symlink",
            idx if self.entries[idx - 1].is_dir() => "Dir",
            _ => "File",
        };
        let entries = self.entries.len();
        let entries_label = if entries == 1 { "Entry" } else { "Entries" };

        write!(
            self.base.info.buff[0].to_mut(),
            "[Files] [{mode}] [{curr_type}] [{curr}/{entries} {entries_label}]",
        )?;

        if let Some(pos) = self.base.sel {
            // Plus 1 since text coordinates are 0 indexed.
            let line = pos.y + 1;
            let col = pos.x + 1;
            write!(
                self.base.info.buff[0].to_mut(),
                " [Selected {line}:{col} - {}:{}]",
                self.base.doc.cur.y + 1,
                self.base.doc.cur.x + 1
            )?;
        }

        Ok(())
    }

    /// Handles self defined view actions.
    fn view_action(&mut self, action: OtherViewAction) -> CommandResult {
        use OtherViewAction::*;

        match action {
            Refresh => match FilesBuffer::load_dir(&self.path, &mut self.entries) {
                Ok(contents) => self.base.doc.set_contents(&contents, 0, 0),
                Err(err) => {
                    return CommandResult::SetAndChangeBuffer(
                        INFO_BUFF_IDX,
                        vec![Cow::from(err.to_string())],
                        None,
                    );
                }
            },
            SelectItem => {
                return self
                    .select_item()
                    .or_else(|err| {
                        Ok::<CommandResult, Error>(CommandResult::SetAndChangeBuffer(
                            INFO_BUFF_IDX,
                            vec![Cow::from(err.to_string())],
                            None,
                        ))
                    })
                    .unwrap();
            }
            ChangeToTextBuffer => change_buffer!(self, TXT_BUFF_IDX),
            ChangeToInfoBuffer => change_buffer!(self, INFO_BUFF_IDX),
        }

        // Rest motion repeat buffer after successful command.
        self.base.motion_repeat.clear();
        CommandResult::Ok
    }

    /// Handles self apply and self defined command ticks.
    fn command_tick(tick: CommandTick<()>) -> CommandResult {
        use CommandTick::*;

        match tick {
            Apply(cmd) => FilesBuffer::apply_command(&cmd),
            Other(()) => unreachable!("Illegal state"),
        }
    }
}

impl Buffer for FilesBuffer {
    fn need_rerender(&self) -> bool {
        self.base.rerender
    }

    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        write!(stdout, "{Hide}{All}")?;
        self.base.rerender = false;

        self.info_line().map_err(Error::other)?;

        let (cursor_style, cmd) = match self.base.mode {
            Mode::View | Mode::Other(()) => (CursorStyle::BlinkingBlock, false),
            Mode::Command => (CursorStyle::BlinkingBar, true),
        };

        if cmd {
            self.base
                .cmd_view
                .render_bar(stdout, &self.base.cmd, COMMAND_PROMPT)?;
        } else {
            self.base
                .info_view
                .render_bar(stdout, &self.base.info, "")?;
        }

        self.base
            .doc_view
            .render_document(stdout, &self.base.doc, self.base.sel)?;

        let (view, prompt) = if cmd {
            (&self.base.cmd_view, Some(COMMAND_PROMPT))
        } else {
            (&self.base.doc_view, None)
        };
        view.render_cursor(stdout, cursor_style, prompt)?;

        stdout.flush()
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.base.doc_view.w == w && self.base.doc_view.h == h {
            return;
        }

        self.base.rerender = true;
        self.base.resize(w, h);
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
                Err(tick) => FilesBuffer::command_tick(tick),
            },
            Mode::Other(()) => unreachable!("Illegal state"),
        }
    }

    fn set_contents(&mut self, _: &[Cow<'static, str>], path: Option<PathBuf>) {
        self.base.doc.set_contents(&[], 0, 0);
        if let Some(path) = path {
            self.path = path;
        }
        self.base.doc_view.cur = Cursor::new(0, 0);

        self.base.sel = None;
        self.base.motion_repeat.clear();
        self.base.matches.clear();

        self.base.rerender = true;
    }

    fn can_quit(&self) -> Result<(), Vec<Cow<'static, str>>> {
        Ok(())
    }
}
