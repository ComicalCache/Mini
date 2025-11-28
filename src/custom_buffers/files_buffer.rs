mod apply_command;
mod interact;

use crate::{
    buffer::{
        Buffer, BufferKind,
        base::{BaseBuffer, Mode},
        edit,
    },
    cursor::{self, Cursor, CursorStyle},
    display::Display,
    document::Document,
    jump,
    message::{Message, MessageKind},
    movement,
    util::Command,
    yank,
};
use std::{io::Error, path::PathBuf};
use termion::event::Key;

enum ViewMode {
    Normal,
    Yank,
}

/// A file browser buffer.
pub struct FilesBuffer {
    base: BaseBuffer<()>,
    view_mode: ViewMode,

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
        let mut entries = Vec::new();
        let contents = Self::load_dir(&path, &mut entries)?;

        Ok(Self {
            base: BaseBuffer::new(w, h, x_off, y_off, Some(contents))?,
            view_mode: ViewMode::Normal,
            info: Document::new(0, 0, None),
            path,
            entries,
        })
    }

    fn refresh(&mut self) -> Command {
        match Self::load_dir(&self.path, &mut self.entries) {
            Ok(contents) => {
                // Set contents moves the doc.cur to the beginning.
                self.base.doc.from(contents.as_str());
                self.base.doc_view.cur = Cursor::new(0, 0);
                self.base.sel = None;

                Command::Ok
            }
            Err(err) => Command::Error(err.to_string()),
        }
    }

    fn selected_remove_command<S: AsRef<str>>(&mut self, cmd: S) -> Command {
        if self.base.doc.cur.y == 0 {
            return Command::Ok;
        }

        // Set the command and move the cursor to be at the end of the input.
        self.base.cmd.from(
            format!(
                "{} {}",
                cmd.as_ref(),
                self.base.doc.line(self.base.doc.cur.y).unwrap()
            )
            .as_str(),
        );
        cursor::jump_to_end_of_line(&mut self.base.cmd, &mut self.base.cmd_view);
        self.base.change_mode(Mode::Command);

        Command::Ok
    }

    /// Creates an info line
    fn info_line(&mut self) {
        use std::fmt::Write;

        let mut info_line = String::new();

        let mode = match self.base.mode {
            Mode::View => " [V]",
            Mode::Command => " [C]",
            Mode::Other(()) => unreachable!(),
        };
        let view_mode = match self.view_mode {
            ViewMode::Normal => "",
            ViewMode::Yank => " [yank]",
        };
        // No plus 1 since the first entry is always ".." and not really a directory entry.
        let curr = self.base.doc.cur.y;
        let curr_type = match curr {
            0 => " [Parent Dir]",
            idx if self.entries[idx - 1].is_symlink() => " [Symlink]",
            idx if self.entries[idx - 1].is_dir() => " [Dir]",
            _ => " [File]",
        };
        let entries = self.entries.len();
        let entries_label = if entries == 1 { "Entry" } else { "Entries" };

        write!(
            &mut info_line,
            "[Files]{mode} [{curr}/{entries} {entries_label}]{curr_type}{view_mode}",
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
    fn view_tick(&mut self, key: Option<Key>) -> Command {
        let Some(key) = key else {
            return Command::Ok;
        };

        match self.view_mode {
            ViewMode::Normal => match key {
                Key::Char('h') | Key::Left => movement!(self, left),
                Key::Char('H') => movement!(self, shift_left),
                Key::Char('j') | Key::Down => movement!(self, down),
                Key::Char('J') => movement!(self, shift_down),
                Key::Char('k') | Key::Up => movement!(self, up),
                Key::Char('K') => movement!(self, shift_up),
                Key::Char('l') | Key::Right => movement!(self, right),
                Key::Char('L') => movement!(self, shift_right),
                Key::Char('w') => movement!(self, next_word),
                Key::Char('W') => movement!(self, next_word_end),
                Key::Char('b') => movement!(self, prev_word),
                Key::Char('B') => movement!(self, prev_word_end),
                Key::Char('s') => movement!(self, next_whitespace),
                Key::Char('S') => movement!(self, prev_whitespace),
                Key::Char('}') => movement!(self, next_empty_line),
                Key::Char('{') => movement!(self, prev_empty_line),
                Key::Char('<') => jump!(self, jump_to_beginning_of_line),
                Key::Char('>') => jump!(self, jump_to_end_of_line),
                Key::Char('.') => jump!(self, jump_to_matching_opposite),
                Key::Char('g') => jump!(self, jump_to_end_of_file),
                Key::Char('G') => jump!(self, jump_to_beginning_of_file),
                Key::Char('v') => self.base.sel = Some(self.base.doc.cur),
                Key::Esc => self.base.sel = None,
                Key::Char('y') => self.view_mode = ViewMode::Yank,
                Key::Char(' ') => self.base.change_mode(Mode::Command),
                Key::Char('n') => self.base.next_match(),
                Key::Char('N') => self.base.prev_match(),
                Key::Char('r') => return self.refresh(),
                Key::Char('\n') => {
                    return self
                        .select_item()
                        .or_else(|err| Ok::<Command, Error>(Command::Error(err.to_string())))
                        .unwrap();
                }
                Key::Char('d') => return self.selected_remove_command("rm"),
                Key::Char('D') => return self.selected_remove_command("rm!"),
                _ => {}
            },
            ViewMode::Yank => {
                match key {
                    Key::Char('v') => yank!(self, selection, SELECTION),
                    Key::Char('y') => yank!(self, line),
                    Key::Char('h') => yank!(self, left, REPEAT),
                    Key::Char('l') => yank!(self, right, REPEAT),
                    Key::Char('w') => yank!(self, next_word, REPEAT),
                    Key::Char('W') => yank!(self, next_word_end, REPEAT),
                    Key::Char('b') => yank!(self, prev_word, REPEAT),
                    Key::Char('B') => yank!(self, prev_word_end, REPEAT),
                    Key::Char('s') => yank!(self, next_whitespace, REPEAT),
                    Key::Char('S') => yank!(self, prev_whitespace, REPEAT),
                    Key::Char('}') => yank!(self, next_empty_line, REPEAT),
                    Key::Char('{') => yank!(self, prev_empty_line, REPEAT),
                    Key::Char('<') => yank!(self, beginning_of_line),
                    Key::Char('>') => yank!(self, end_of_line),
                    Key::Char('.') => yank!(self, matching_opposite),
                    Key::Char('g') => yank!(self, end_of_file),
                    Key::Char('G') => yank!(self, beginning_of_file),
                    _ => {}
                }
                self.view_mode = ViewMode::Normal;
            }
        }

        Command::Ok
    }

    /// Handles self apply and self defined command ticks.
    fn command_tick(&mut self, key: Option<Key>) -> Command {
        let Some(key) = key else {
            return Command::Ok;
        };

        match key {
            Key::Esc => self.base.change_mode(Mode::View),
            Key::Left => cursor::left(&mut self.base.cmd, &mut self.base.cmd_view, 1),
            Key::Right => cursor::right(&mut self.base.cmd, &mut self.base.cmd_view, 1),
            Key::Up => self.base.prev_command_history(),
            Key::Down => self.base.next_command_history(),
            Key::AltRight => cursor::next_word(&mut self.base.cmd, &mut self.base.cmd_view, 1),
            Key::AltLeft => cursor::prev_word(&mut self.base.cmd, &mut self.base.cmd_view, 1),
            Key::Char('\n') => {
                // Commands have only one line.
                let cmd = self.base.cmd.line(0).unwrap().to_string();
                if !cmd.is_empty() {
                    self.base.cmd_history.push(cmd.clone());
                }
                self.base.change_mode(Mode::View);

                match self.base.apply_command(cmd) {
                    Ok(res) => return res,
                    Err(cmd) => return self.apply_command(&cmd),
                }
            }
            Key::Char('\t') => {
                edit::write_tab(&mut self.base.cmd, &mut self.base.cmd_view, None, false);
            }
            Key::Backspace => edit::delete_char(&mut self.base.cmd, &mut self.base.cmd_view, None),
            Key::Char(ch) => {
                edit::write_char(&mut self.base.cmd, &mut self.base.cmd_view, None, ch);
            }
            _ => {}
        }

        Command::Ok
    }
}

impl Buffer for FilesBuffer {
    fn kind(&self) -> BufferKind {
        BufferKind::Files
    }

    fn name(&self) -> String {
        unreachable!()
    }

    fn need_rerender(&self) -> bool {
        self.base.rerender
    }

    fn render(&mut self, display: &mut Display) {
        self.base.rerender = false;

        self.info_line();

        let (cursor_style, cmd) = match self.base.mode {
            Mode::View | Mode::Other(()) => (CursorStyle::SteadyBlock, false),
            Mode::Command => (CursorStyle::BlinkingBar, true),
        };

        self.base.doc_view.render_gutter(display, &self.base.doc);
        self.base
            .doc_view
            .render_document(display, &self.base.doc, self.base.sel);

        if cmd {
            self.base.cmd_view.render_bar(
                self.base.cmd.line(0).unwrap().to_string().trim_end(),
                0,
                display,
                &self.base.cmd,
            );
        } else {
            self.base.info_view.render_bar(
                self.info.line(0).unwrap().to_string().trim_end(),
                0,
                display,
                &self.info,
            );
        }

        if let Some(message) = &self.base.message {
            self.base.doc_view.render_message(display, message);
            self.base
                .doc_view
                .render_cursor(display, CursorStyle::Hidden);
            return;
        }

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

    fn tick(&mut self, key: Option<Key>) -> Command {
        // Only rerender if input was received.
        self.base.rerender |= key.is_some();

        // Intercept inputs if a message is shown.
        if let Some(message) = &mut self.base.message
            && let Some(key) = key
        {
            match key {
                Key::Char('J') => {
                    if message.scroll + 1 < message.lines {
                        message.scroll += 1;
                        self.base.rerender = true;
                    }

                    return Command::Ok;
                }
                Key::Char('K') => {
                    message.scroll = message.scroll.saturating_sub(1);
                    self.base.rerender = true;
                    return Command::Ok;
                }
                Key::Char('Y') => {
                    if let Err(err) = self.base.clipboard.set_text(message.text.clone()) {
                        return Command::Error(err.to_string());
                    }

                    return Command::Info("Message yanked to clipboard".to_string());
                }
                // Clear the message on any other key press.
                _ => self.base.clear_message(),
            }
        }

        match self.base.mode {
            Mode::View => self.view_tick(key),
            Mode::Command => self.command_tick(key),
            Mode::Other(()) => unreachable!(),
        }
    }

    fn get_message(&self) -> Option<Message> {
        self.base.message.clone()
    }

    fn set_message(&mut self, kind: MessageKind, text: String) {
        self.base.set_message(kind, text);
    }

    fn can_quit(&self) -> Result<(), String> {
        Ok(())
    }
}
