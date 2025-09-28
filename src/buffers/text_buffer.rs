mod apply_cmd;
mod edit;
mod r#move;

use crate::{
    INFO_BUFF_IDX,
    buffer::Buffer,
    cursor::Cursor,
    document::Document,
    util::{CommandResult, CursorStyle, read_file_to_lines},
    viewport::Viewport,
};
use std::{
    fs::File,
    io::{BufWriter, Error, Stdout},
};
use termion::{event::Key, raw::RawTerminal};

#[derive(Clone, Copy)]
enum Mode {
    View,
    Write,
    Command,
}

pub struct TextBuffer {
    doc: Document,
    cmd: Document,
    view: Viewport,
    file: Option<File>,
    selected_pos: Option<Cursor>,
    mode: Mode,
    motion_repeat: String,
}

impl TextBuffer {
    pub fn new(w: usize, h: usize, mut file: Option<File>) -> Result<Self, Error> {
        let content = if let Some(file) = file.as_mut() {
            Some(read_file_to_lines(file)?)
        } else {
            None
        };

        Ok(TextBuffer {
            doc: Document::new(content, 0, 0),
            cmd: Document::new(None, 0, 0),
            view: Viewport::new(w, h, 0, h / 2),
            file,
            selected_pos: None,
            mode: Mode::View,
            motion_repeat: String::new(),
        })
    }

    fn info_line(&self) -> String {
        use std::fmt::Write;

        let mut info_line = String::new();

        let mode = match self.mode {
            Mode::View => "V",
            Mode::Write => "W",
            Mode::Command => "C",
        };
        // Plus 1 since text coordinates are 0 indexed.
        let line = self.doc.cursor.y + 1;
        let col = self.doc.cursor.x + 1;
        let total = self.doc.lines.len();
        let percentage = 100 * line / total;
        let size: usize = self.doc.lines.iter().map(String::len).sum();

        write!(
            &mut info_line,
            "[Text] [{mode}] [{line}:{col}/{total} {percentage}%] [{size}B]",
        )
        .unwrap();
        if let Some(pos) = self.selected_pos {
            // Plus 1 since text coordinates are 0 indexed.
            let line = pos.y + 1;
            let col = pos.x + 1;
            write!(&mut info_line, " [Selected {line}:{col}]").unwrap();
        }

        let edited = if self.doc.edited { '*' } else { ' ' };
        write!(&mut info_line, " {edited}").unwrap();

        info_line
    }

    fn cmd_line(&self) -> Option<(String, Cursor)> {
        match self.mode {
            Mode::Command => Some((self.cmd.lines[0].clone(), self.cmd.cursor)),
            _ => None,
        }
    }

    fn change_mode(&mut self, mode: Mode) {
        match self.mode {
            Mode::Command => {
                // Clear command line so its ready for next entry.
                self.cmd.lines[0].clear();
                self.cmd.cursor = Cursor::new(0, 0);

                // Set cursor to the beginning of line so its always at a predictable position.
                // TODO: restore prev position.
                self.jump_to_beginning_of_line();
            }
            Mode::View | Mode::Write => {}
        }

        match mode {
            Mode::Command => {
                // Set cursor to the beginning of line to avoid weird scrolling behaviour.
                // TODO: save curr position and restore.
                self.jump_to_beginning_of_line();
            }
            Mode::View | Mode::Write => {}
        }

        self.mode = mode;
    }
}

impl Buffer for TextBuffer {
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error> {
        let cursor_style = match self.mode {
            Mode::View => CursorStyle::BlinkingBlock,
            Mode::Write | Mode::Command => CursorStyle::BlinkingBar,
        };

        self.view.render(
            stdout,
            &self.doc,
            self.info_line(),
            self.cmd_line(),
            cursor_style,
        )
    }

    fn resize(&mut self, w: usize, h: usize) {
        if self.view.w == w && self.view.h == h {
            return;
        }

        self.view.resize(w, h, self.view.cursor.x.min(w), h / 2);
    }

    fn tick(&mut self, key: Option<Key>) -> CommandResult {
        let Some(key) = key else {
            return CommandResult::Ok;
        };

        match self.mode {
            Mode::View => match key {
                Key::Char('i') => self.change_mode(Mode::Write),
                Key::Char('a') => {
                    self.right(1);
                    self.change_mode(Mode::Write);
                }
                Key::Char('o') => {
                    self.insert_move_new_line_bellow();
                    self.change_mode(Mode::Write);
                }
                Key::Char('O') => {
                    self.insert_move_new_line_above();
                    self.change_mode(Mode::Write);
                }
                Key::Char('h') => self.left(self.motion_repeat.parse::<usize>().unwrap_or(1)),
                Key::Char('j') => self.down(self.motion_repeat.parse::<usize>().unwrap_or(1)),
                Key::Char('k') => self.up(self.motion_repeat.parse::<usize>().unwrap_or(1)),
                Key::Char('l') => self.right(self.motion_repeat.parse::<usize>().unwrap_or(1)),
                Key::Char('w') => {
                    for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                        self.next_word();
                    }
                }
                Key::Char('b') => {
                    for _ in 0..self.motion_repeat.parse::<usize>().unwrap_or(1) {
                        self.prev_word();
                    }
                }
                Key::Char('<') => {
                    self.motion_repeat.clear();
                    self.jump_to_beginning_of_line();
                }
                Key::Char('>') => {
                    self.motion_repeat.clear();
                    self.jump_to_end_of_line();
                }
                Key::Char('.') => {
                    self.motion_repeat.clear();
                    self.jump_to_matching_opposite();
                }
                Key::Char('g') => {
                    self.motion_repeat.clear();
                    self.jump_to_end_of_file();
                }
                Key::Char('G') => {
                    self.motion_repeat.clear();
                    self.jump_to_beginning_of_file();
                }
                Key::Char('?') => {
                    self.motion_repeat.clear();
                    return CommandResult::ChangeBuffer(INFO_BUFF_IDX);
                }
                Key::Char(' ') => self.change_mode(Mode::Command),
                Key::Char('v') => self.selected_pos = Some(self.doc.cursor),
                Key::Esc => self.selected_pos = None,
                Key::Char('d') => self.delete_selection(),
                Key::Char(ch) if ch.is_ascii_digit() => {
                    self.motion_repeat.push(ch);

                    // Skip resetting the motion repeat buffer while entering the amount.
                    return CommandResult::Ok;
                }
                _ => {}
            },
            Mode::Write => match key {
                Key::Esc => self.change_mode(Mode::View),
                Key::Left => self.left(1),
                Key::Down => self.down(1),
                Key::Up => self.up(1),
                Key::Right => self.right(1),
                Key::AltLeft => self.prev_word(),
                Key::AltRight => self.next_word(),
                Key::Char('\n') => self.write_new_line_char(),
                Key::Char('\t') => self.write_tab(),
                Key::Char(ch) => self.write_char(ch),
                Key::Backspace => self.delete_char(),
                _ => {}
            },
            Mode::Command => match key {
                Key::Esc => self.change_mode(Mode::View),
                Key::Left => self.cmd_left(1),
                Key::Right => self.cmd_right(1),
                Key::Char('\n') => {
                    let res = self.apply_cmd();
                    self.change_mode(Mode::View);
                    return res;
                }
                Key::Char('\t') => self.write_cmd_tab(),
                Key::Char(ch) => self.write_cmd_char(ch),
                // TODO: support Delete key in the future
                Key::Backspace => self.delete_cmd_char(),
                _ => {}
            },
        }

        // Rest motion repeat buffer after successful command.
        self.motion_repeat.clear();
        CommandResult::Ok
    }

    fn set_contents(&mut self, contents: &[String]) {
        self.doc.set_contents(contents, 0, 0);
    }
}
