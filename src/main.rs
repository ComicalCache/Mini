#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::too_many_lines, clippy::similar_names)]

mod buffer;
mod buffer_impls;
mod buffer_manager;
mod cursor;
mod display;
mod document;
mod history;
mod message;
mod selection;
mod shell_command;
mod util;
mod viewport;

use crate::{
    buffer_manager::BufferManager,
    display::Display,
    util::{file_name, open_file},
};
use polling::{Events, Poller};
use std::{io::BufWriter, os::fd::AsFd, time::Duration};
use termion::{
    input::TermRead,
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};

// Random value chosen by dev-rng.
const STDIN_EVENT_KEY: usize = 25663;
const INFO_MSG: &str = include_str!("../info.txt");

fn main() {
    let mut args = std::env::args();
    args.next();

    let path = args.next();
    if let Some(path) = &path
        && path == "--help"
    {
        let version = option_env!("CARGO_PKG_VERSION").or(Some("?.?.?")).unwrap();
        println!("Mini - A terminal text-editor (v{version})\n\n{INFO_MSG}");
        return;
    }

    print!("{ToAlternateScreen}");
    let res = mini(path.as_ref());
    print!("{ToMainScreen}");

    if let Err(err) = res {
        eprintln!("{err}");
    }
}

fn mini(path: Option<&String>) -> Result<(), std::io::Error> {
    let (file, file_name) = path.as_ref().map_or((None, None), |path| {
        (Some(open_file(path)), file_name(path))
    });

    // Setup stdin and stdout.
    let mut stdout = BufWriter::new(std::io::stdout().into_raw_mode()?);
    let stdin = std::io::stdin();
    let mut stdin_keys = std::io::stdin().keys();

    // Use polling to periodically read stdin.
    let poller = Poller::new()?;
    unsafe { poller.add(&stdin.as_fd(), polling::Event::readable(STDIN_EVENT_KEY))? };

    let (w, h) = termion::terminal_size()?;

    let mut buffer_manager = BufferManager::new(path, file, file_name, w as usize, h as usize)?;
    let mut display = Display::new(w as usize, h as usize);

    buffer_manager.render(&mut display);
    display.draw(&mut stdout)?;

    let mut events = Events::new();
    loop {
        let (w, h) = termion::terminal_size()?;
        buffer_manager.resize(w as usize, h as usize);
        display.resize(w as usize, h as usize);

        // Clear previous iterations events and fetch new ones.
        events.clear();
        poller.wait(&mut events, Some(Duration::from_millis(20)))?;

        let key = if events.iter().any(|e| e.key == STDIN_EVENT_KEY) {
            // If a new event exists, send a tick with the key immediately.
            match stdin_keys.next() {
                Some(Ok(key)) => Some(key),
                Some(Err(_)) | None => return Ok(()),
            }
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

    Ok(())
}
