mod state;
mod util;

use crate::{
    state::State,
    util::{CursorMove, Mode, read_file},
};
use std::{fs::OpenOptions, io::Write};
use termion::{
    event::Key,
    input::TermRead,
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), std::io::Error> {
    let mut args = std::env::args();
    args.next();

    let mut file = if let Some(path) = args.next() {
        if path == "--help" {
            println!(" Mini terminal text editor, run with file(path) argument to open or create");
            println!("   Press h | j | k | l (or ← | ↓ | ↑ | →) to move the cursor");
            println!("   Press w to skip the the next word");
            println!("   Press b to go back one word");
            println!("   Press < | > to jump to the beginning/end of a line");
            println!("   Press . to jump to the matching opposite bracket");
            println!("   Press e to enter the error buffer");
            println!("   Press 1 | 2 | 3 | 4 | 5 to switch between different buffers");
            println!("   Press space to enter command mode");
            println!("     Write q to quit");
            println!("     Write w to write the buffer to file");
            println!("     Write w <path> to write this/all future writes to the specified path");
            println!("     Write o <path> to open a file and replace the buffer");
            println!("   Press esc to exit command mode");
            println!("   Press i to enter write mode");
            println!("   Press a to enter write mode one character after the current");
            println!("   Press esc to exit write mode");
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

    let (width, height) = termion::terminal_size()?;
    let mut states = [
        // Buffer 1 (Error)
        State::new(width as usize, height as usize, vec![String::new()], None),
        // Buffer 2 (Default)
        State::new(width as usize, height as usize, line_buff, file),
        // Buffer 3
        State::new(width as usize, height as usize, vec![String::new()], None),
        // Buffer 4
        State::new(width as usize, height as usize, vec![String::new()], None),
        // Buffer 5
        State::new(width as usize, height as usize, vec![String::new()], None),
        // Buffer 6
        State::new(width as usize, height as usize, vec![String::new()], None),
    ];
    let mut curr_state = 1;

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().into_raw_mode()?;

    write!(&mut stdout, "{ToAlternateScreen}")?;
    stdout.flush()?;

    states[curr_state].print_screen(&mut stdout, curr_state)?;

    for key in stdin.keys() {
        let key = key?;
        match states[curr_state].mode() {
            Mode::View => match key {
                Key::Char('i') => states[curr_state].change_mode(Mode::Write),
                Key::Char('a') => {
                    states[curr_state].move_cursor(CursorMove::Right, 1);
                    states[curr_state].change_mode(Mode::Write);
                }
                Key::Char('o') => {
                    states[curr_state].insert_move_new_line_bellow();
                    states[curr_state].change_mode(Mode::Write);
                }
                Key::Char('O') => {
                    states[curr_state].insert_move_new_line_above();
                    states[curr_state].change_mode(Mode::Write);
                }
                Key::Char('h') | Key::Left => states[curr_state].move_cursor(CursorMove::Left, 1),
                Key::Char('j') | Key::Down => states[curr_state].move_cursor(CursorMove::Down, 1),
                Key::Char('k') | Key::Up => states[curr_state].move_cursor(CursorMove::Up, 1),
                Key::Char('l') | Key::Right => states[curr_state].move_cursor(CursorMove::Right, 1),
                Key::Char('w') => states[curr_state].next_word(),
                Key::Char('b') => states[curr_state].prev_word(),
                Key::Char('<') => states[curr_state].jump_to_start_of_line(),
                Key::Char('>') => states[curr_state].jump_to_end_of_line(),
                Key::Char('.') => states[curr_state].jump_to_matching_opposite(),
                Key::Char(' ') => states[curr_state].change_mode(Mode::Command),
                Key::Char('e') => curr_state = 0,
                Key::Char('1') => curr_state = 1,
                Key::Char('2') => curr_state = 2,
                Key::Char('3') => curr_state = 3,
                Key::Char('4') => curr_state = 4,
                Key::Char('5') => curr_state = 5,
                _ => {}
            },
            Mode::Write => match key {
                Key::Esc => states[curr_state].change_mode(Mode::View),
                Key::Left => states[curr_state].move_cursor(CursorMove::Left, 1),
                Key::Down => states[curr_state].move_cursor(CursorMove::Down, 1),
                Key::Up => states[curr_state].move_cursor(CursorMove::Up, 1),
                Key::Right => states[curr_state].move_cursor(CursorMove::Right, 1),
                Key::Char('\n') => states[curr_state].write_new_line(),
                Key::Char('\t') => states[curr_state].write_tab(),
                Key::Char(ch) => states[curr_state].write_char(ch),
                Key::Backspace => states[curr_state].delete_char(),
                _ => {}
            },
            Mode::Command => match key {
                Key::Esc => states[curr_state].change_mode(Mode::View),
                Key::Left => states[curr_state].move_cmd_cursor(CursorMove::Left, 1),
                Key::Right => states[curr_state].move_cmd_cursor(CursorMove::Right, 1),
                Key::Char('\n') => {
                    let prev_state = curr_state;

                    match states[curr_state].apply_cmd()? {
                        util::CmdResult::Quit => break,
                        util::CmdResult::Continue => {}
                        util::CmdResult::Error(err) => {
                            // Write error to error buffer
                            curr_state = 0;
                            states[curr_state].set_line_buff(&err);
                        }
                    }

                    states[prev_state].change_mode(Mode::View);
                }
                Key::Char('\t') => states[curr_state].write_cmd_tab(),
                Key::Char(ch) => states[curr_state].write_cmd_char(ch),
                Key::Backspace => states[curr_state].delete_cmd_char(),
                _ => {}
            },
        }

        // Print new state after every input
        states[curr_state].print_screen(&mut stdout, curr_state)?;
    }

    write!(stdout, "{ToMainScreen}")?;
    stdout.flush()
}
