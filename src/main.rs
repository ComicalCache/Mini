mod state;
mod util;

use crate::{
    state::State,
    util::{CursorMove, Mode},
};
use std::fs::OpenOptions;
use termion::{event::Key, input::TermRead};

fn main() -> Result<(), std::io::Error> {
    let mut args = std::env::args();
    args.next();
    let Some(path) = args.next() else {
        println!("Expected `--help` or file(path)");
        return Ok(());
    };

    if path == "--help" {
        println!(" Mini terminal text editor, run with file(path) argument to open or create");
        println!("   Press space to enter command mode");
        println!("     Write q to quit");
        println!("     Write w to write the buffer to file");
        println!("   Press h | j | k | l (or ← | ↓ | ↑ | →) to move the cursor");
        println!("   Press i to enter write mode");
        println!("   Press a to enter write mode one character after the current");
        println!("   Press esc to exit write mode");
        println!("   Press w to skip the the next word");
        println!("   Press b to go back one word");
        println!("   Press < | > to jump to the beginning/end of a line");
        println!("   Press . to jump to the matching opposite bracket");
        return Ok(());
    }

    let mut state = {
        // Avoid leaking variables into outer scope
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let (width, height) = termion::terminal_size()?;
        State::new(width as usize, height as usize, file)
    }?;
    let stdin = std::io::stdin();

    // Initialize state and print the initial view
    state.init()?;
    state.print_screen()?;

    for key in stdin.keys() {
        let key = key?;
        match state.mode() {
            Mode::View => match key {
                Key::Char('i') => state.change_mode(Mode::Write),
                Key::Char('a') => {
                    state.move_cursor(CursorMove::Right, 1);
                    state.change_mode(Mode::Write);
                }
                Key::Char('o') => {
                    state.insert_move_new_line_bellow();
                    state.change_mode(Mode::Write);
                }
                Key::Char('O') => {
                    state.insert_move_new_line_above();
                    state.change_mode(Mode::Write);
                }
                Key::Char('h') | Key::Left => state.move_cursor(CursorMove::Left, 1),
                Key::Char('j') | Key::Down => state.move_cursor(CursorMove::Down, 1),
                Key::Char('k') | Key::Up => state.move_cursor(CursorMove::Up, 1),
                Key::Char('l') | Key::Right => state.move_cursor(CursorMove::Right, 1),
                Key::Char('w') => state.next_word(),
                Key::Char('b') => state.prev_word(),
                Key::Char('<') => state.jump_to_start_of_line(),
                Key::Char('>') => state.jump_to_end_of_line(),
                Key::Char('.') => state.jump_to_matching_opposite(),
                Key::Char(' ') => state.change_mode(Mode::Command),
                _ => {}
            },
            Mode::Write => match key {
                Key::Esc => state.change_mode(Mode::View),
                Key::Left => state.move_cursor(CursorMove::Left, 1),
                Key::Down => state.move_cursor(CursorMove::Down, 1),
                Key::Up => state.move_cursor(CursorMove::Up, 1),
                Key::Right => state.move_cursor(CursorMove::Right, 1),
                Key::Char('\n') => state.write_new_line(),
                Key::Char('\t') => state.write_tab(),
                Key::Char(ch) => state.write_char(ch),
                Key::Backspace => state.delete_char(),
                _ => {}
            },
            Mode::Command => match key {
                Key::Esc => state.change_mode(Mode::View),
                Key::Left => state.move_cmd_cursor(CursorMove::Left, 1),
                Key::Right => state.move_cmd_cursor(CursorMove::Right, 1),
                Key::Char('\n') => match state.apply_cmd()? {
                    util::CmdResult::Quit => break,
                    util::CmdResult::Continue => {}
                },
                Key::Char('\t') => state.write_cmd_tab(),
                Key::Char(ch) => state.write_cmd_char(ch),
                Key::Backspace => state.delete_cmd_char(),
                _ => unimplemented!(),
            },
        }

        // Print new state after every input
        state.print_screen()?;
    }

    state.deinit()
}
