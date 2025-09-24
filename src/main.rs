mod buffer;
mod util;

use crate::{
    buffer::Buffer,
    util::{CursorMove, Mode, read_file},
};
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
};
use termion::{
    event::Key,
    input::TermRead,
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};

macro_rules! r#move {
    ($repeat_buff:ident, $buffers:ident, $buffer:ident, $dir:expr) => {{
        $buffers[$buffer].move_cursor($dir, $repeat_buff.parse::<usize>().unwrap_or(1));
        $repeat_buff.clear();
    }};
}

macro_rules! skip {
    ($repeat_buff:ident, $buffers:ident, $buffer:ident, $method:ident) => {{
        for _ in 0..$repeat_buff.parse::<usize>().unwrap_or(1) {
            $buffers[$buffer].$method();
        }
        $repeat_buff.clear();
    }};
}

const INFO_BUFF: usize = 0;
const TXT_BUFF: usize = 1;

const INFO_MSG: &str = include_str!("info.txt");

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), std::io::Error> {
    let mut args = std::env::args();
    args.next();

    let mut file = if let Some(path) = args.next() {
        if path == "--help" {
            println!("{INFO_MSG}");
            return Ok(());
        }

        Some(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)?,
        )
    } else {
        None
    };
    let line_buff = if let Some(file) = file.as_mut() {
        read_file(file)?
    } else {
        vec![String::new()]
    };

    let stdin = std::io::stdin();
    let mut stdout = BufWriter::new(std::io::stdout().into_raw_mode()?);

    // Init buffers
    let (width, height) = termion::terminal_size()?;
    let mut buffers = [
        // Buffer 1 (Info)
        Buffer::new(width as usize, height as usize, vec![String::new()], None),
        // Buffer 2 (Text)
        Buffer::new(width as usize, height as usize, line_buff, file),
    ];
    let mut buffer = TXT_BUFF;

    // Init terminal by switching to alternate screen
    write!(&mut stdout, "{ToAlternateScreen}")?;
    buffers[buffer].print_screen(&mut stdout, "Text")?;
    stdout.flush()?;

    // Repeat buffer to execute motions multiple times
    let mut repeat_buff = String::new();
    for key in stdin.keys() {
        let (width, height) = termion::terminal_size()?;
        for buff in &mut buffers {
            buff.update_screen_dimentions(width as usize, height as usize);
        }

        let key = key?;
        match buffers[buffer].mode() {
            Mode::View => match key {
                // Can't edit error buffer
                Key::Char('i') if buffer != INFO_BUFF => buffers[buffer].change_mode(Mode::Write),
                // Can't edit error buffer
                Key::Char('a') if buffer != INFO_BUFF => {
                    buffers[buffer].move_cursor(CursorMove::Right, 1);
                    buffers[buffer].change_mode(Mode::Write);
                }
                // Can't edit error buffer
                Key::Char('o') if buffer != INFO_BUFF => {
                    buffers[buffer].insert_move_new_line_bellow();
                    buffers[buffer].change_mode(Mode::Write);
                }
                // Can't edit error buffer
                Key::Char('O') if buffer != INFO_BUFF => {
                    buffers[buffer].insert_move_new_line_above();
                    buffers[buffer].change_mode(Mode::Write);
                }
                Key::Char('h') => r#move!(repeat_buff, buffers, buffer, CursorMove::Left),
                Key::Char('j') => r#move!(repeat_buff, buffers, buffer, CursorMove::Down),
                Key::Char('k') => r#move!(repeat_buff, buffers, buffer, CursorMove::Up),
                Key::Char('l') => r#move!(repeat_buff, buffers, buffer, CursorMove::Right),
                Key::Char('w') => skip!(repeat_buff, buffers, buffer, next_word),
                Key::Char('b') => skip!(repeat_buff, buffers, buffer, prev_word),
                Key::Char('<') => skip!(repeat_buff, buffers, buffer, jump_to_start_of_line),
                Key::Char('>') => skip!(repeat_buff, buffers, buffer, jump_to_end_of_line),
                Key::Char('.') => skip!(repeat_buff, buffers, buffer, jump_to_matching_opposite),
                Key::Char('g') => skip!(repeat_buff, buffers, buffer, jump_to_end),
                Key::Char('G') => skip!(repeat_buff, buffers, buffer, jump_to_start),
                // Can't command in error buffer
                Key::Char(' ') if buffer != INFO_BUFF => buffers[buffer].change_mode(Mode::Command),
                Key::Char('?') if buffer == INFO_BUFF => buffer = TXT_BUFF,
                Key::Char('?') if buffer == TXT_BUFF => buffer = INFO_BUFF,
                Key::Char(ch) if ch.is_ascii_digit() => repeat_buff.push(ch),
                // Can't select in error buffer
                Key::Char('v') if buffer != INFO_BUFF => buffers[buffer].set_select(),
                Key::Esc => buffers[buffer].reset_select(),
                // Can't delete in error buffer
                Key::Char('d') if buffer != INFO_BUFF => buffers[buffer].delete_selection(false),
                Key::Char('D') if buffer != INFO_BUFF => buffers[buffer].delete_selection(true),
                _ => {}
            },
            Mode::Write => match key {
                Key::Esc => buffers[buffer].change_mode(Mode::View),
                Key::Left => buffers[buffer].move_cursor(CursorMove::Left, 1),
                Key::Down => buffers[buffer].move_cursor(CursorMove::Down, 1),
                Key::Up => buffers[buffer].move_cursor(CursorMove::Up, 1),
                Key::Right => buffers[buffer].move_cursor(CursorMove::Right, 1),
                Key::Char('\n') => buffers[buffer].write_new_line(),
                Key::Char('\t') => buffers[buffer].write_tab(),
                Key::Char(ch) => buffers[buffer].write_char(ch),
                Key::Backspace => buffers[buffer].delete_char(),
                _ => {}
            },
            Mode::Command => match key {
                Key::Esc => buffers[buffer].change_mode(Mode::View),
                Key::Left => buffers[buffer].move_cmd_cursor(CursorMove::Left, 1),
                Key::Right => buffers[buffer].move_cmd_cursor(CursorMove::Right, 1),
                Key::Char('\n') => {
                    let res = buffers[buffer].apply_cmd();
                    buffers[buffer].change_mode(Mode::View);

                    match res {
                        util::CmdResult::Quit => break,
                        // Reset error buffer on successful command
                        util::CmdResult::Continue => buffers[INFO_BUFF].set_line_buff(""),
                        util::CmdResult::Info(err) => {
                            // Write error to error buffer
                            buffer = INFO_BUFF;
                            buffers[buffer].set_line_buff(&err);
                        }
                    }
                }
                Key::Char('\t') => buffers[buffer].write_cmd_tab(),
                Key::Char(ch) => buffers[buffer].write_cmd_char(ch),
                // TODO: support Delete key in the future
                Key::Backspace => buffers[buffer].delete_cmd_char(),
                _ => {}
            },
        }

        // Print new buffer after every input
        let name = if buffer == INFO_BUFF { "Info" } else { "Text" };
        buffers[buffer].print_screen(&mut stdout, name)?;
    }

    write!(stdout, "{ToMainScreen}")?;
    stdout.flush()
}
