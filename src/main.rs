mod cursor;
mod document;
mod text_buffer;
mod util;
mod viewport;

use polling::{Events, Poller};
use std::{
    io::{BufWriter, Write},
    os::fd::AsFd,
    time::Duration,
};
use termion::{
    event::Key,
    input::TermRead,
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};

use crate::{
    text_buffer::TextBuffer,
    util::{CommandResult, open_file},
};

// Random value chosen by me
const STDIN_EVENT_KEY: usize = 25663;
const INFO_MSG: &str = include_str!("info.txt");

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), std::io::Error> {
    let mut args = std::env::args();
    args.next();

    let file = if let Some(path) = args.next() {
        if path == "--help" {
            println!("{INFO_MSG}");
            return Ok(());
        }

        Some(open_file(path)?)
    } else {
        None
    };

    // Setup stdin and stdout
    let stdin = std::io::stdin();
    let stdin_fd = stdin.as_fd();
    let mut stdout = BufWriter::new(std::io::stdout().into_raw_mode()?);
    let mut stdin_keys = std::io::stdin().keys();

    // Use polling to periodically read stdin
    let poller = Poller::new()?;
    unsafe { poller.add(&stdin_fd, polling::Event::readable(STDIN_EVENT_KEY))? };

    let (w, h) = termion::terminal_size()?;
    let mut txt_buff = TextBuffer::new(w as usize, h as usize, file)?;

    // Init terminal by switching to alternate screen
    write!(&mut stdout, "{ToAlternateScreen}")?;
    txt_buff.render(&mut stdout)?;
    stdout.flush()?;

    let mut quit = false;
    let mut events = Events::new();
    let mut keys: Vec<Key> = Vec::new();
    while !quit {
        events.clear();
        let mut num_events = poller.wait(&mut events, Some(Duration::from_millis(750)))?;

        if num_events == 0 && !keys.is_empty() {
            // No keystroke in the last input period -> submit multi-key input
            keys.clear();
        } else if events.iter().any(|e| e.key == STDIN_EVENT_KEY) {
            num_events = events.iter().filter(|e| e.key == STDIN_EVENT_KEY).count();
            if num_events != 0 {
                let key = stdin_keys.next().unwrap()?;

                // Buffer multi-key commands if not the first key or single key command doesn't exist
                if keys.is_empty() {
                    match txt_buff.single_key_command(key) {
                        CommandResult::Ok => {}
                        CommandResult::Quit => quit = true,
                        CommandResult::NotFound => keys.push(key),
                    }
                } else {
                    keys.push(key);
                }
            }
        }

        txt_buff.render(&mut stdout)?;
        poller.modify(stdin_fd, polling::Event::readable(STDIN_EVENT_KEY))?;
    }

    write!(stdout, "{ToMainScreen}")?;
    stdout.flush()
}
