mod buffers;
mod cursor;
mod document;
mod traits;
mod util;
mod viewport;

use crate::{
    buffers::{info_buffer::InfoBuffer, text_buffer::TextBuffer},
    traits::Buffer,
    util::{CommandResult, open_file},
};
use polling::{Events, Poller};
use std::{
    io::{BufWriter, Write},
    os::fd::AsFd,
    time::Duration,
};
use termion::{
    input::TermRead,
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};

// Random value chosen by dev-rng.
const STDIN_EVENT_KEY: usize = 25663;
const INFO_MSG: &str = include_str!("info.txt");

// Indices of buffers.
const TXT_BUFF_IDX: usize = 0;
const INFO_BUFF_IDX: usize = 1;

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

    // Setup stdin and stdout.
    let mut stdout = BufWriter::new(std::io::stdout().into_raw_mode()?);
    let stdin = std::io::stdin();
    let stdin_fd = stdin.as_fd();
    let mut stdin_keys = std::io::stdin().keys();

    // Use polling to periodically read stdin.
    let poller = Poller::new()?;
    unsafe { poller.add(&stdin_fd, polling::Event::readable(STDIN_EVENT_KEY))? };

    let (w, h) = termion::terminal_size()?;

    // Create array of buffers that can be switched to.
    let mut buffs: [Box<dyn Buffer>; 2] = [
        Box::new(TextBuffer::new(w as usize, h as usize, file)?),
        Box::new(InfoBuffer::new(w as usize, h as usize)),
    ];
    let mut curr_buff = TXT_BUFF_IDX;

    // Init terminal by switching to alternate screen.
    write!(&mut stdout, "{ToAlternateScreen}")?;
    buffs[curr_buff].render(&mut stdout)?;
    stdout.flush()?;

    let mut quit = false;
    let mut events = Events::new();
    while !quit {
        // Handle terminal resizing.
        let (w, h) = termion::terminal_size()?;
        for buff in &mut buffs {
            buff.resize(w as usize, h as usize);
        }

        // Clear previous iterations events and fetch new ones.
        events.clear();
        poller.wait(&mut events, Some(Duration::from_millis(25)))?;

        // If a new event exists, send a tick with the key immediately.
        if events.iter().any(|e| e.key == STDIN_EVENT_KEY) {
            match buffs[curr_buff].tick(Some(stdin_keys.next().unwrap()?)) {
                CommandResult::Ok => {}
                CommandResult::Info(info) => {
                    buffs[INFO_BUFF_IDX].set_contents(&info);
                    curr_buff = INFO_BUFF_IDX;
                }
                CommandResult::ChangeBuffer(idx) => curr_buff = idx,
                CommandResult::Quit => quit = true,
            }
        }
        // Otherwise send an empty tick after the timeout.
        else {
            match buffs[curr_buff].tick(None) {
                CommandResult::Ok => {}
                CommandResult::Info(info) => {
                    buffs[INFO_BUFF_IDX].set_contents(&info);
                    curr_buff = INFO_BUFF_IDX;
                }
                CommandResult::ChangeBuffer(idx) => curr_buff = idx,
                CommandResult::Quit => quit = true,
            }
        }

        // Render the "new" state and re-enable polling.
        buffs[curr_buff].render(&mut stdout)?;
        poller.modify(stdin_fd, polling::Event::readable(STDIN_EVENT_KEY))?;
    }

    // Switch back to the main screen before exiting.
    write!(stdout, "{ToMainScreen}")?;
    stdout.flush()
}
