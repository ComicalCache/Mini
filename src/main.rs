#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::too_many_lines, clippy::enum_glob_use, clippy::similar_names)]
#![feature(trait_alias)]

mod buffer;
mod buffer_manager;
mod cursor;
mod custom_buffers;
mod display;
mod document;
mod history;
mod message;
mod shell_command;
mod util;
mod viewport;

use crate::{
    buffer_manager::BufferManager,
    display::Display,
    util::{file_name, open_file},
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
const INFO_MSG: &str = include_str!("../info.txt");

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), std::io::Error> {
    let mut args = std::env::args();
    args.next();

    // Print help or read file if argument was supplied.
    let path = args.next();
    let (file, file_name) = if let Some(path) = &path {
        if path == "--help" {
            let version = option_env!("CARGO_PKG_VERSION").or(Some("?.?.?")).unwrap();
            println!("Mini - A terminal text-editor (v{version})\n\n{INFO_MSG}");
            return Ok(());
        }

        let file = open_file(path);
        (Some(file), file_name(path))
    } else {
        (None, None)
    };

    // Setup stdin and stdout.
    let mut stdout = BufWriter::new(std::io::stdout().into_raw_mode()?);
    let stdin = std::io::stdin();
    let mut stdin_keys = std::io::stdin().keys();

    // Use polling to periodically read stdin.
    let poller = Poller::new()?;
    unsafe { poller.add(&stdin.as_fd(), polling::Event::readable(STDIN_EVENT_KEY))? };

    let (w, h) = termion::terminal_size()?;

    // Buffer manager holds app state.
    let mut buffer_manager =
        BufferManager::new(path.as_ref(), file, file_name, w as usize, h as usize)?;
    // Create a display buffer.
    let mut display = Display::new(w as usize, h as usize);

    // Init terminal by switching to alternate screen.
    write!(&mut stdout, "{ToAlternateScreen}")?;
    buffer_manager.render(&mut display);
    display.draw(&mut stdout)?;

    let mut events = Events::new();
    loop {
        // Handle terminal resizing.
        let (w, h) = termion::terminal_size()?;
        buffer_manager.resize(w as usize, h as usize);
        display.resize(w as usize, h as usize);

        // Clear previous iterations events and fetch new ones.
        events.clear();
        poller.wait(&mut events, Some(Duration::from_millis(20)))?;

        let key = if events.iter().any(|e| e.key == STDIN_EVENT_KEY) {
            // If a new event exists, send a tick with the key immediately.
            Some(stdin_keys.next().unwrap()?)
        } else {
            // Otherwise send an empty tick after the timeout.
            None
        };

        if !buffer_manager.tick(key) {
            break;
        }
        buffer_manager.render(&mut display);
        display.draw(&mut stdout)?;

        // Re-enable polling.
        poller.modify(stdin.as_fd(), polling::Event::readable(STDIN_EVENT_KEY))?;
    }

    // Switch back to the main screen before exiting.
    write!(stdout, "{ToMainScreen}")?;
    stdout.flush()
}
