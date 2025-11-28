mod apply_command;
mod history;
mod insert;

use crate::{
    buffer::{
        Buffer, BufferKind,
        base::{BaseBuffer, Mode},
        delete, edit,
    },
    change,
    cursor::{self, CursorStyle},
    delete,
    display::Display,
    document::Document,
    history::History,
    jump,
    message::{Message, MessageKind},
    movement,
    util::Command,
    yank,
};
use std::{
    fs::File,
    io::{Error, Read},
};
use termion::event::Key;

enum OtherMode {
    Write,
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
    base: BaseBuffer<OtherMode>,
    view_mode: ViewMode,

    /// The info bar content.
    info: Document,

    /// The opened file.
    file: Option<File>,
    /// The name of the opened file.
    file_name: Option<String>,

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
            view_mode: ViewMode::Normal,
            info: Document::new(0, 0, None),
            file,
            file_name,
            history: History::new(),
        })
    }

    /// Creates an info line
    fn info_line(&mut self) {
        use std::fmt::Write;

        let mut info_line = String::new();

        let mode = match self.base.mode {
            Mode::View => "[V] ",
            Mode::Command => "[C] ",
            Mode::Other(OtherMode::Write) => "[W] ",
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
            "{mode}[{line}:{col}] [{line}/{total} {percentage}%] [{size}B]{view_mode}",
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

        let edited = if self.base.doc.edited { '*' } else { ' ' };
        write!(&mut info_line, " {edited}").unwrap();

        self.info.from(info_line.as_str());
    }

    /// Handles self defined view actions.
    fn view_tick(&mut self, key: Option<Key>) -> Command {
        use OtherMode::Write;

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
                Key::Char('i') => self.base.change_mode(Mode::Other(Write)),
                Key::Char('a') => {
                    cursor::right(&mut self.base.doc, &mut self.base.doc_view, 1);
                    self.base.change_mode(Mode::Other(Write));
                }
                Key::Char('A') => {
                    cursor::jump_to_end_of_line(&mut self.base.doc, &mut self.base.doc_view);
                    self.base.change_mode(Mode::Other(Write));
                }
                Key::Char('o') => {
                    self.insert_move_new_line_bellow();
                    self.base.change_mode(Mode::Other(Write));
                }
                Key::Char('O') => {
                    self.insert_move_new_line_above();
                    self.base.change_mode(Mode::Other(Write));
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
                            &mut self.base.doc_view,
                            &mut self.base.sel,
                            Some(&mut self.history),
                        );
                        self.base.change_mode(Mode::Other(Write));
                    }
                    Key::Char('c') => {
                        cursor::jump_to_beginning_of_line(
                            &mut self.base.doc,
                            &mut self.base.doc_view,
                        );
                        delete::end_of_line(
                            &mut self.base.doc,
                            &mut self.base.doc_view,
                            Some(&mut self.history),
                        );
                        self.base.change_mode(Mode::Other(Write));
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

        Command::Ok
    }

    /// Handles write mode ticks.
    fn write_tick(&mut self, key: Option<Key>) -> Command {
        let Some(key) = key else {
            return Command::Ok;
        };

        match key {
            Key::Esc => self.base.change_mode(Mode::View),
            Key::Left => cursor::left(&mut self.base.doc, &mut self.base.doc_view, 1),
            Key::Down => cursor::down(&mut self.base.doc, &mut self.base.doc_view, 1),
            Key::Up => cursor::up(&mut self.base.doc, &mut self.base.doc_view, 1),
            Key::Right => cursor::right(&mut self.base.doc, &mut self.base.doc_view, 1),
            Key::AltRight => cursor::next_word(&mut self.base.doc, &mut self.base.doc_view, 1),
            Key::AltLeft => cursor::prev_word(&mut self.base.doc, &mut self.base.doc_view, 1),
            Key::Char('\t') => edit::write_tab(
                &mut self.base.doc,
                &mut self.base.doc_view,
                Some(&mut self.history),
                true,
            ),
            Key::Backspace => edit::delete_char(
                &mut self.base.doc,
                &mut self.base.doc_view,
                Some(&mut self.history),
            ),
            Key::Char(ch) => edit::write_char(
                &mut self.base.doc,
                &mut self.base.doc_view,
                Some(&mut self.history),
                ch,
            ),
            _ => {}
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

        self.info_line();

        let (cursor_style, cmd) = match self.base.mode {
            Mode::View => (CursorStyle::BlinkingBlock, false),
            Mode::Command => (CursorStyle::BlinkingBar, true),
            Mode::Other(OtherMode::Write) => (CursorStyle::BlinkingBar, false),
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
            Mode::Other(OtherMode::Write) => self.write_tick(key),
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
