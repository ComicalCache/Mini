mod buffer;
mod util;

use crate::{
    buffer::Buffer,
    util::{CursorMove, Mode, read_file},
};
use std::{fs::OpenOptions, io::Write};
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

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), std::io::Error> {
    let mut args = std::env::args();
    args.next();

    let mut file = if let Some(path) = args.next() {
        if path == "--help" {
            println!(" Mini terminal text editor, run with file(path) argument to open or create");
            println!("   Press h | j | k | l to move the cursor");
            println!("   Press w to skip the the next word");
            println!("   Press b to go back one word");
            println!("   Press < | > to jump to the beginning/end of a line");
            println!("   Press . to jump to the matching opposite bracket");
            println!("   Press e to switch between the error and text buffer");
            println!("   Press space to enter command mode");
            println!("     Write q to quit");
            println!("     Write w to write the buffer to file");
            println!("     Write w <path> to write this/all future writes to the specified path");
            println!("     Write o <path> to open a file and replace the buffer");
            println!(
                "     Write oo <path> to open a file and replace the buffer, discarding unsaved changes"
            );
            println!("   Press esc to exit command mode");
            println!("   Press i to enter write mode");
            println!("   Press a to enter write mode one character after the current");
            println!("   Press o to enter write mode one line under the current");
            println!("   Press O to enter write mode one line above the current");
            println!("   Press g to go to the end of the file");
            println!("   Press G to go to the start of the file");
            println!("   Press esc to exit write mode");
            println!("   Type a number followed by a motion to execute it multiple times");
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
    let mut stdout = std::io::stdout().into_raw_mode()?;

    // Init terminal by switching to alternate screen
    write!(&mut stdout, "{ToAlternateScreen}")?;
    stdout.flush()?;

    // Init buffers
    let (width, height) = termion::terminal_size()?;
    let mut buffers = [
        // Buffer 1 (Error)
        Buffer::new(width as usize, height as usize, vec![String::new()], None),
        // Buffer 2 (Default)
        Buffer::new(width as usize, height as usize, line_buff, file),
    ];
    let mut buffer = 1;
    buffers[buffer].print_screen(&mut stdout, "Text Buffer")?;

    // Repeat buffer to execute motions multiple times
    let mut repeat_buff = String::new();

    for key in stdin.keys() {
        let key = key?;
        match buffers[buffer].mode() {
            Mode::View => match key {
                // Can't edit error buffer
                Key::Char('i') if buffer != 0 => buffers[buffer].change_mode(Mode::Write),
                // Can't edit error buffer
                Key::Char('a') if buffer != 0 => {
                    buffers[buffer].move_cursor(CursorMove::Right, 1);
                    buffers[buffer].change_mode(Mode::Write);
                }
                // Can't edit error buffer
                Key::Char('o') if buffer != 0 => {
                    buffers[buffer].insert_move_new_line_bellow();
                    buffers[buffer].change_mode(Mode::Write);
                }
                // Can't edit error buffer
                Key::Char('O') if buffer != 0 => {
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
                Key::Char(' ') if buffer != 0 => buffers[buffer].change_mode(Mode::Command),
                Key::Char('e') if buffer == 0 => buffer = 1,
                Key::Char('e') if buffer == 1 => buffer = 0,
                Key::Char(ch) if '0' <= ch && ch <= '9' => repeat_buff.push(ch),
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
                    let res = buffers[buffer].apply_cmd()?;
                    buffers[buffer].change_mode(Mode::View);

                    match res {
                        util::CmdResult::Quit => break,
                        util::CmdResult::Continue => {}
                        util::CmdResult::Error(err) => {
                            // Write error to error buffer
                            buffer = 0;
                            buffers[buffer].set_line_buff(&err);
                        }
                    }
                }
                Key::Char('\t') => buffers[buffer].write_cmd_tab(),
                Key::Char(ch) => buffers[buffer].write_cmd_char(ch),
                // Maybe support Delete key in the future
                Key::Backspace => buffers[buffer].delete_cmd_char(),
                _ => {}
            },
        }

        // Print new buffer after every input
        if buffer == 0 {
            buffers[buffer].print_screen(&mut stdout, "Error")?;
        } else {
            buffers[buffer].print_screen(&mut stdout, "Text Buffer")?;
        }
    }

    write!(stdout, "{ToMainScreen}")?;
    stdout.flush()
}
