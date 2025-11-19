mod apply_command;
mod interact;

use crate::{
    INFO_BUFF_IDX, TXT_BUFF_IDX,
    buffer::{
        Buffer,
        base::{BaseBuffer, COMMAND_PROMPT, CommandTick, Mode, ViewAction},
    },
    c_buff,
    cursor::{self, Cursor},
    display::Display,
    document::Document,
    sc_buff,
    util::{CommandResult, CursorStyle},
};
use std::{io::Error, path::PathBuf};
use termion::event::Key;

#[derive(Clone, Copy)]
enum OtherViewAction {
    // Interact
    Refresh,
    SelectItem,
    Remove,
    RecursiveRemove,

    // Buffers
    ChangeToTextBuffer,
    ChangeToInfoBuffer,
}

/// A file browser buffer.
pub struct FilesBuffer {
    base: BaseBuffer<(), OtherViewAction, ()>,

    /// The info bar content.
    info: Document,

    /// The path of the current item.
    path: PathBuf,
    /// All entries of the dir containing the current item.
    entries: Vec<PathBuf>,
}

impl FilesBuffer {
    pub fn new(
        w: usize,
        h: usize,
        x_off: usize,
        y_off: usize,
        path: PathBuf,
    ) -> Result<Self, Error> {
        use OtherViewAction::*;

        let mut entries = Vec::new();
        let contents = Self::load_dir(&path, &mut entries)?;

        let mut base = BaseBuffer::new(w, h, x_off, y_off, Some(contents))?;
        base.view_state_machine.command_map = base
            .view_state_machine
            .command_map
            .simple(Key::Char('t'), ViewAction::Other(ChangeToTextBuffer))
            .simple(Key::Char('?'), ViewAction::Other(ChangeToInfoBuffer))
            .simple(Key::Char('r'), ViewAction::Other(Refresh))
            .simple(Key::Char('\n'), ViewAction::Other(SelectItem))
            .simple(Key::Char('d'), ViewAction::Other(Remove))
            .simple(Key::Char('D'), ViewAction::Other(RecursiveRemove));

        Ok(Self {
            base,
            info: Document::new(0, 0, None),
            path,
            entries,
        })
    }

    fn refresh(&mut self) -> CommandResult {
        match Self::load_dir(&self.path, &mut self.entries) {
            Ok(contents) => {
                // Set contents moves the doc.cur to the beginning.
                self.base.doc.set_contents(contents);
                self.base.doc_view.cur = Cursor::new(0, 0);
                self.base.sel = None;

                CommandResult::Ok
            }
            Err(err) => sc_buff!(self, INFO_BUFF_IDX, err.to_string()),
        }
    }

    fn selected_remove_command<S: AsRef<str>>(&mut self, cmd: S) -> CommandResult {
        if self.base.doc.cur.y == 0 {
            return CommandResult::Ok;
        }

        // Set the command and move the cursor to be at the end of the input.
        *self.base.cmd.buff[0].to_mut() = format!(
            "{} {}",
            cmd.as_ref(),
            self.base.doc.buff[self.base.doc.cur.y]
        );
        cursor::jump_to_end_of_line(&mut self.base.cmd, &mut self.base.cmd_view);
        self.base.change_mode(Mode::Command);

        CommandResult::Ok
    }

    /// Creates an info line
    fn info_line(&mut self) {
        use std::fmt::Write;

        self.info.buff[0].to_mut().clear();

        let mode = match self.base.mode {
            Mode::View => "V",
            Mode::Command => "C",
            Mode::Other(()) => unreachable!(),
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
            self.info.buff[0].to_mut(),
            " [Files] [{mode}] [{curr}/{entries} {entries_label}] [{curr_type}]",
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
                self.info.buff[0].to_mut(),
                " [Selected {}:{} - {}:{}]",
                start.y + 1,
                start.x + 1,
                end.y + 1,
                end.x + 1
            )
            .unwrap();
        }
    }

    /// Handles self defined view actions.
    fn view_action(&mut self, action: OtherViewAction) -> CommandResult {
        use OtherViewAction::*;

        match action {
            Refresh => self.refresh(),
            SelectItem => self
                .select_item()
                .or_else(|err| {
                    Ok::<CommandResult, Error>(sc_buff!(self, INFO_BUFF_IDX, err.to_string()))
                })
                .unwrap(),
            Remove => self.selected_remove_command("rm"),
            RecursiveRemove => self.selected_remove_command("rm!"),
            ChangeToTextBuffer => c_buff!(self, TXT_BUFF_IDX),
            ChangeToInfoBuffer => c_buff!(self, INFO_BUFF_IDX),
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

impl Buffer for FilesBuffer {
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
                }
            }

            let contents = &self.base.doc.contiguous_buff;
            self.base.doc.highlighter.highlight(contents);
        }
    }

    fn render(&mut self, display: &mut Display) {
        self.base.rerender = false;

        self.info_line();

        let (cursor_style, cmd) = match self.base.mode {
            Mode::View | Mode::Other(()) => (CursorStyle::SteadyBlock, false),
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
            self.base
                .info_view
                .render_bar(&self.info.buff[0], 0, display, &self.info, "");
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

    fn set_contents(&mut self, _: String, path: Option<PathBuf>, _: Option<String>) {
        // Set contents moves the doc.cur to the beginning.
        self.base.doc.set_contents(String::new());
        if let Some(path) = path {
            self.path = path;
        }
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
