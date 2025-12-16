mod apply_command;
mod history;
mod insert;

use crate::{
    buffer::{Buffer, BufferKind, BufferResult, base::BaseBuffer, delete, edit},
    change,
    cursor::{self, CursorStyle},
    delete,
    display::Display,
    document::Document,
    history::History,
    jump,
    message::{Message, MessageKind},
    movement,
    selection::SelectionKind,
    shell_command::{ShellCommand, ShellCommandResult},
    shift, yank,
};
use std::{
    fs::File,
    io::{Error, Read},
    sync::mpsc::TryRecvError,
};
use termion::event::Key;

enum Mode {
    View,
    Command,
    Insert,
}

enum ViewMode {
    Normal,
    Yank,
    Delete,
    Change,
    Replace,
}

/// A text buffer.
pub struct TextBuffer {
    base: BaseBuffer,
    mode: Mode,
    view_mode: ViewMode,

    /// The info bar content.
    info: Document,

    /// The opened file.
    file: Option<File>,
    /// The name of the opened file.
    file_name: Option<String>,

    /// A runner handling command execution.
    shell_command: Option<ShellCommand>,

    /// A history of edits to undo and redo.
    history: History,
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

        Ok(Self {
            base: BaseBuffer::new(w, h, x_off, y_off, contents)?,
            mode: Mode::View,
            view_mode: ViewMode::Normal,
            info: Document::new(0, 0, None),
            file,
            file_name,
            shell_command: None,
            history: History::new(),
        })
    }

    /// Changes the mode.
    fn change_mode(&mut self, new_mode: Mode) {
        match self.mode {
            Mode::Command => {
                // Clear command line so its ready for next entry. Don't save contents here since they are only
                // saved when hitting enter.
                self.base.cmd.from("");
                self.base.cmd_view.scroll_x = 0;
                self.base.cmd_view.scroll_y = 0;
            }
            Mode::View => {
                // Since search matches could have been overwritten we discard all matches.
                if self.base.doc.edited {
                    self.base.clear_matches();
                }
            }
            Mode::Insert => {}
        }

        match new_mode {
            Mode::Command => self.base.cmd_history_idx = self.base.cmd_history.len(),
            Mode::View | Mode::Insert => {}
        }

        self.mode = new_mode;
    }

    /// Creates an info line
    fn info_line(&mut self) {
        use std::fmt::Write;

        let mut info_line = String::new();

        let mode = match self.mode {
            Mode::View => "[VIS] ",
            Mode::Insert => "[INS] ",
            Mode::Command => unreachable!(),
        };
        let view_mode = match self.view_mode {
            ViewMode::Normal => "",
            ViewMode::Yank => " [yank]",
            ViewMode::Delete => " [delete]",
            ViewMode::Change => " [change]",
            ViewMode::Replace => " [replace]",
        };
        // Plus 1 since text coordinates are 0 indexed.
        let line = self.base.doc.cur.y + 1;
        let col = self.base.doc.cur.x + 1;
        let total = self.base.doc.len();
        let percentage = 100 * line / total;
        let size: usize = self.base.doc.lines().map(|l| l.bytes().len()).sum();

        if self.file.is_some() {
            write!(&mut info_line, "[{}] ", self.file_name.as_ref().unwrap()).unwrap();
        }

        write!(
            &mut info_line,
            "{mode}[{line}:{col}/{total} {percentage}%] [{size}B]{view_mode}",
        )
        .unwrap();

        match self.base.selections.len() {
            0 => {}
            1 => write!(&mut info_line, " [1 selection]").unwrap(),
            n => write!(&mut info_line, " [{n} selections]").unwrap(),
        }

        if let Some(shell_command) = &self.shell_command {
            match shell_command.cmd.split_whitespace().next() {
                Some(cmd) => write!(&mut info_line, " [Command '{cmd}' running]",).unwrap(),
                None => write!(&mut info_line, " [Command running]",).unwrap(),
            }
        }

        let edited = if self.base.doc.edited { '*' } else { ' ' };
        write!(&mut info_line, " {edited}").unwrap();

        self.info.from(info_line.as_str());
    }

    /// Handles self defined view actions.
    fn view_tick(&mut self, key: Option<Key>) -> BufferResult {
        let Some(key) = key else {
            return BufferResult::Ok;
        };

        match self.view_mode {
            ViewMode::Normal => match key {
                Key::Char('h') | Key::Left => movement!(self, left),
                Key::Char('H') => shift!(self, shift_left),
                Key::Char('j') | Key::Down => movement!(self, down),
                Key::Char('J') => shift!(self, shift_down),
                Key::Char('k') | Key::Up => movement!(self, up),
                Key::Char('K') => shift!(self, shift_up),
                Key::Char('l') | Key::Right => movement!(self, right),
                Key::Char('L') => shift!(self, shift_right),
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
                Key::Char('v') => {
                    self.base.add_selection(SelectionKind::Normal);
                    self.base.update_selection();
                }
                Key::Char('V') => {
                    self.base.add_selection(SelectionKind::Line);
                    self.base.update_selection();
                }
                Key::Esc => self.base.selections.clear(),
                Key::Char('y') => self.view_mode = ViewMode::Yank,
                Key::Char(' ') => self.change_mode(Mode::Command),
                Key::Char('n') => self.base.next_match(),
                Key::Char('N') => self.base.prev_match(),
                Key::Char('i') => self.change_mode(Mode::Insert),
                Key::Char('a') => {
                    cursor::right(&mut self.base.doc, 1);
                    self.change_mode(Mode::Insert);
                }
                Key::Char('A') => {
                    cursor::jump_to_end_of_line(&mut self.base.doc);
                    self.change_mode(Mode::Insert);
                }
                Key::Char('o') => {
                    self.insert_move_new_line_bellow();
                    self.change_mode(Mode::Insert);
                }
                Key::Char('O') => {
                    self.insert_move_new_line_above();
                    self.change_mode(Mode::Insert);
                }
                Key::Char('d') => self.view_mode = ViewMode::Delete,
                Key::Char('x') => delete!(self, right, REPEAT),
                Key::Char('c') => self.view_mode = ViewMode::Change,
                Key::Char('p') => {
                    if let Some(res) = self.paste(false, false) {
                        return res;
                    }
                    self.base.clear_matches();
                }
                Key::Char('P') => {
                    self.insert_move_new_line_above();
                    if let Some(res) = self.paste(true, false) {
                        return res;
                    }
                    self.base.clear_matches();
                }
                Key::Char('r') => self.view_mode = ViewMode::Replace,
                Key::Char('u') => self.undo(),
                Key::Char('U') => self.redo(),
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
            ViewMode::Delete => {
                match key {
                    Key::Char('l') => delete!(self, right, REPEAT),
                    Key::Char('v') => delete!(self, selection, SELECTION),
                    Key::Char('d') => delete!(self, line, REPEAT),
                    Key::Char('h') => delete!(self, left, REPEAT),
                    Key::Char('w') => delete!(self, next_word, REPEAT),
                    Key::Char('b') => delete!(self, prev_word, REPEAT),
                    Key::Char('W') => delete!(self, next_word_end, REPEAT),
                    Key::Char('B') => delete!(self, prev_word_end, REPEAT),
                    Key::Char('s') => delete!(self, next_whitespace, REPEAT),
                    Key::Char('S') => delete!(self, prev_whitespace, REPEAT),
                    Key::Char('}') => delete!(self, next_empty_line, REPEAT),
                    Key::Char('{') => delete!(self, prev_empty_line, REPEAT),
                    Key::Char('<') => delete!(self, beginning_of_line),
                    Key::Char('>') => delete!(self, end_of_line),
                    Key::Char('.') => delete!(self, matching_opposite),
                    Key::Char('g') => delete!(self, end_of_file),
                    Key::Char('G') => delete!(self, beginning_of_file),
                    _ => {}
                }
                self.view_mode = ViewMode::Normal;
            }
            ViewMode::Change => {
                match key {
                    Key::Char('v') => {
                        delete::selection(
                            &mut self.base.doc,
                            &mut self.base.selections,
                            Some(&mut self.history),
                        );
                        self.change_mode(Mode::Insert);
                    }
                    Key::Char('c') => {
                        cursor::jump_to_beginning_of_line(&mut self.base.doc);
                        delete::end_of_line(&mut self.base.doc, Some(&mut self.history));
                        self.change_mode(Mode::Insert);
                    }
                    Key::Char('h') => change!(self, left, REPEAT),
                    Key::Char('l') => change!(self, right, REPEAT),
                    Key::Char('w') => change!(self, next_word, REPEAT),
                    Key::Char('b') => change!(self, prev_word, REPEAT),
                    Key::Char('W') => change!(self, next_word_end, REPEAT),
                    Key::Char('B') => change!(self, prev_word_end, REPEAT),
                    Key::Char('s') => change!(self, next_whitespace, REPEAT),
                    Key::Char('S') => change!(self, prev_whitespace, REPEAT),
                    Key::Char('}') => change!(self, next_empty_line, REPEAT),
                    Key::Char('{') => change!(self, prev_empty_line, REPEAT),
                    Key::Char('<') => change!(self, beginning_of_line),
                    Key::Char('>') => change!(self, end_of_line),
                    Key::Char('.') => change!(self, matching_opposite),
                    Key::Char('g') => change!(self, end_of_file),
                    Key::Char('G') => change!(self, beginning_of_file),
                    _ => {}
                }
                self.view_mode = ViewMode::Normal;
            }
            ViewMode::Replace => {
                if let Key::Char(ch) = key {
                    self.replace(ch);
                    self.base.clear_matches();
                }
                self.view_mode = ViewMode::Normal;
            }
        }

        BufferResult::Ok
    }

    /// Handles write mode ticks.
    fn write_tick(&mut self, key: Option<Key>) -> BufferResult {
        let Some(key) = key else {
            return BufferResult::Ok;
        };

        match key {
            Key::Esc => self.change_mode(Mode::View),
            Key::Left => cursor::left(&mut self.base.doc, 1),
            Key::Down => cursor::down(&mut self.base.doc, 1),
            Key::Up => cursor::up(&mut self.base.doc, 1),
            Key::Right => cursor::right(&mut self.base.doc, 1),
            Key::AltRight => cursor::next_word(&mut self.base.doc, 1),
            Key::AltLeft => cursor::prev_word(&mut self.base.doc, 1),
            Key::Char('\t') => edit::write_tab(&mut self.base.doc, Some(&mut self.history), true),
            Key::Backspace => edit::delete_char(&mut self.base.doc, Some(&mut self.history)),
            Key::Char(ch) => edit::write_char(&mut self.base.doc, Some(&mut self.history), ch),
            _ => {}
        }

        BufferResult::Ok
    }

    /// Handles self apply and self defined command ticks.
    fn command_tick(&mut self, key: Option<Key>) -> BufferResult {
        let Some(key) = key else {
            return BufferResult::Ok;
        };

        match key {
            Key::Esc => self.change_mode(Mode::View),
            Key::Left => cursor::left(&mut self.base.cmd, 1),
            Key::Right => cursor::right(&mut self.base.cmd, 1),
            Key::Up => self.base.prev_command_history(),
            Key::Down => self.base.next_command_history(),
            Key::AltRight => cursor::next_word(&mut self.base.cmd, 1),
            Key::AltLeft => cursor::prev_word(&mut self.base.cmd, 1),
            Key::Char('\n') => {
                // Commands have only one line.
                let cmd = self.base.cmd.line(0).unwrap().to_string();
                if !cmd.is_empty() {
                    self.base.cmd_history.push(cmd.clone());
                }
                self.change_mode(Mode::View);

                match self.base.apply_command(cmd) {
                    Ok(res) => return res,
                    Err(cmd) => return self.apply_command(&cmd),
                }
            }
            Key::Char('\t') => edit::write_tab(&mut self.base.cmd, None, false),
            Key::Backspace => edit::delete_char(&mut self.base.cmd, None),
            Key::Char(ch) => edit::write_char(&mut self.base.cmd, None, ch),
            _ => {}
        }

        BufferResult::Ok
    }

    fn shell_tick(&mut self, key: Option<Key>) -> BufferResult {
        let shell_command = &mut *self.shell_command.as_mut().unwrap();

        // Greedily read as much as possible.
        loop {
            match shell_command.rx.try_recv() {
                Ok(res) => match res {
                    ShellCommandResult::Data(data) => {
                        self.base.rerender = true;
                        shell_command.parser.process(&data);
                    }
                    ShellCommandResult::Error(err) => {
                        self.base.rerender = true;
                        self.base.doc.append_str(shell_command.contents().as_str());
                        jump!(self, jump_to_end_of_file);

                        self.shell_command = None;
                        return BufferResult::Error(err);
                    }
                    ShellCommandResult::Eof => {
                        self.base.rerender = true;
                        self.base.doc.append_str(shell_command.contents().as_str());
                        jump!(self, jump_to_end_of_file);

                        let res = BufferResult::Info(format!("'{}' finished", shell_command.cmd));
                        self.shell_command = None;
                        return res;
                    }
                },
                // Ignore empty error since we're waiting on data.
                Err(TryRecvError::Empty) => break,
                Err(err) => {
                    self.shell_command = None;
                    return BufferResult::Error(err.to_string());
                }
            }
        }

        // Send key as input if available.
        if let Some(key) = key {
            // Always quit command on 'ctrl+q'.
            if Key::Ctrl('q') == key {
                self.base.rerender = true;
                self.base.doc.append_str(shell_command.contents().as_str());
                jump!(self, jump_to_end_of_file);

                let res = BufferResult::Info(format!("Quit '{}'", shell_command.cmd));
                self.shell_command = None;
                return res;
            } else if let Err(err) = shell_command.write(key) {
                self.base.rerender = true;
                self.base.doc.append_str(shell_command.contents().as_str());
                jump!(self, jump_to_end_of_file);

                self.shell_command = None;
                return BufferResult::Error(err.to_string());
            }
        }

        BufferResult::Ok
    }
}

impl Buffer for TextBuffer {
    fn kind(&self) -> BufferKind {
        BufferKind::Text
    }

    fn name(&self) -> String {
        self.file_name
            .as_ref()
            .map_or_else(|| "Scratchpad".to_string(), Clone::clone)
    }

    fn need_rerender(&self) -> bool {
        self.base.rerender
    }

    fn render(&mut self, display: &mut Display) {
        self.base.rerender = false;

        let (cursor_style, cmd) = match self.mode {
            Mode::View => (CursorStyle::SteadyBlock, false),
            Mode::Command => (CursorStyle::SteadyBar, true),
            Mode::Insert => (CursorStyle::SteadyBar, false),
        };

        self.base.doc_view.recalculate_viewport(&self.base.doc);
        if let Some(shell_command) = &self.shell_command {
            self.base
                .doc_view
                .render_terminal(display, &shell_command.parser);
        } else {
            self.base.doc_view.render_gutter(display, &self.base.doc);
            self.base
                .doc_view
                .render_document(display, &self.base.doc, &self.base.selections);
        }

        if cmd {
            self.base.cmd_view.recalculate_viewport(&self.base.cmd);

            self.base.cmd_view.render_bar(
                self.base.cmd.line(0).unwrap().to_string().trim_end(),
                0,
                display,
            );
        } else {
            self.base.info_view.recalculate_viewport(&self.info);
            self.info_line();

            self.base.info_view.render_bar(
                self.info.line(0).unwrap().to_string().trim_end(),
                0,
                display,
            );
        }

        if let Some(message) = &self.base.message {
            self.base.doc_view.render_message(display, message);
            self.base
                .doc_view
                .render_cursor(display, &self.base.doc, CursorStyle::Hidden);
            return;
        }

        // The shell handles it's own cursor.
        if self.shell_command.is_none() {
            let (view, doc) = if cmd {
                (&self.base.cmd_view, &self.base.cmd)
            } else {
                (&self.base.doc_view, &self.base.doc)
            };
            view.render_cursor(display, doc, cursor_style);
        }
    }

    fn resize(&mut self, w: usize, h: usize, x_off: usize, y_off: usize) {
        self.base.resize(w, h, x_off, y_off);
        if let Some(shell_command) = &mut self.shell_command {
            shell_command.resize(self.base.doc_view.buff_w, self.base.doc_view.h);
        }
    }

    fn tick(&mut self, key: Option<Key>) -> BufferResult {
        // If an active shell command is running, check for updates and paste them at the end of the buffer.
        if self.shell_command.is_some() {
            return self.shell_tick(key);
        }

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

                    return BufferResult::Ok;
                }
                Key::Char('K') => {
                    message.scroll = message.scroll.saturating_sub(1);
                    self.base.rerender = true;
                    return BufferResult::Ok;
                }
                Key::Char('Y') => {
                    if let Err(err) = self.base.clipboard.set_text(message.text.clone()) {
                        return BufferResult::Error(err.to_string());
                    }

                    return BufferResult::Info("Message yanked to clipboard".to_string());
                }
                // Clear the message on any other key press.
                _ => self.base.clear_message(),
            }
        }

        match self.mode {
            Mode::View => self.view_tick(key),
            Mode::Command => self.command_tick(key),
            Mode::Insert => self.write_tick(key),
        }
    }

    fn get_message(&self) -> Option<Message> {
        self.base.message.clone()
    }

    fn set_message(&mut self, kind: MessageKind, text: String) {
        self.base.set_message(kind, text);
    }

    fn can_quit(&self) -> Result<(), String> {
        if !self.base.doc.edited {
            return Ok(());
        }

        Err("There are unsaved changes in the text buffer".to_string())
    }
}
