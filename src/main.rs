#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::too_many_lines, clippy::enum_glob_use, clippy::similar_names)]
#![feature(trait_alias)]

mod buffer;
mod cursor;
mod custom_buffers;
mod display;
mod document;
mod history;
mod message;
mod util;
mod viewport;

use crate::{
    buffer::Buffer,
    custom_buffers::{files_buffer::FilesBuffer, text_buffer::TextBuffer},
    display::Display,
    util::{CommandResult, file_name, open_file},
};
use polling::{Events, Poller};
use std::{
    io::{BufWriter, ErrorKind, Write},
    os::fd::AsFd,
    path::PathBuf,
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

// Indices of buffers.
const TXT_BUFF_IDX: usize = 0;
const FILES_BUFF_IDX: usize = 1;

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
    let base = if let Some(path) = &path {
        // Get the absolute path.
        let mut base = std::fs::canonicalize(PathBuf::from(path))?;

        if !base.is_dir() {
            base.pop();
        }

        base
    } else {
        std::env::current_dir()?
    };

    // Create a display buffer.
    let mut display = Display::new(w as usize, h as usize);

    // Setting the current buffer and error in case of file opening error.
    let mut files_buffer = Box::new(FilesBuffer::new(w as usize, h as usize, 0, 0, base)?);
    let mut curr_buff = if let Some(Err(err)) = &file {
        // Open the `FilesBuffer` if a directory was specified as argument.
        if err.kind() == ErrorKind::IsADirectory {
            FILES_BUFF_IDX
        } else {
            files_buffer.set_message(err.to_string());
            TXT_BUFF_IDX
        }
    } else {
        TXT_BUFF_IDX
    };

    // Vector of buffers.
    let mut buffs: [Box<dyn Buffer>; 2] = [
        Box::new(TextBuffer::new(
            w as usize,
            h as usize,
            0,
            0,
            file.and_then(std::result::Result::ok),
            file_name,
        )?),
        files_buffer,
    ];

    // Init terminal by switching to alternate screen.
    write!(&mut stdout, "{ToAlternateScreen}")?;
    buffs[curr_buff].render(&mut display);
    display.draw(&mut stdout)?;

    let mut quit = false;
    let mut events = Events::new();
    while !quit {
        // Handle terminal resizing.
        let (w, h) = termion::terminal_size()?;
        for buff in &mut buffs {
            buff.resize(w as usize, h as usize, 0, 0);
        }
        display.resize(w as usize, h as usize);

        // Clear previous iterations events and fetch new ones.
        events.clear();
        poller.wait(&mut events, Some(Duration::from_millis(100)))?;

        let key = if events.iter().any(|e| e.key == STDIN_EVENT_KEY) {
            // If a new event exists, send a tick with the key immediately.
            Some(stdin_keys.next().unwrap()?)
        } else {
            // Otherwise send an empty tick after the timeout.
            None
        };

        let mut rerender_changed_buff = false;
        match buffs[curr_buff].tick(key) {
            CommandResult::Ok => {}
            // Change to a different buffer.
            CommandResult::Change(idx) => {
                rerender_changed_buff = true;
                curr_buff = idx;
            }
            // Set a buffer and change to it if the buffer has no pending changes.
            CommandResult::Info(contents) => {
                buffs[curr_buff].set_message(contents);
            }
            // Set a buffer and change to it if the buffer has no pending changes.
            CommandResult::Init(idx, contents, path, file_name) => {
                if let Err(err) = buffs[idx].can_quit() {
                    buffs[curr_buff].set_message(err);
                } else {
                    buffs[idx].set_contents(contents, path, file_name);
                    curr_buff = idx;
                }
            }
            // Quit the app if there are no unsaved changes left.
            CommandResult::Quit => {
                quit = true;

                for idx in 0..buffs.len() {
                    if let Err(err) = buffs[idx].can_quit() {
                        curr_buff = idx;
                        buffs[curr_buff].set_message(err);

                        quit = false;
                        break;
                    }
                }
            }
            // Quit discarding all pending changes.
            CommandResult::ForceQuit => {
                quit = true;
            }
        }

        // Render the "new" state if necessary.
        if !quit && (rerender_changed_buff || buffs[curr_buff].need_rerender()) {
            buffs[curr_buff].render(&mut display);
            display.draw(&mut stdout)?;
        }
        // Re-enable polling.
        poller.modify(stdin.as_fd(), polling::Event::readable(STDIN_EVENT_KEY))?;
    }

    // Switch back to the main screen before exiting.
    write!(stdout, "{ToMainScreen}")?;
    stdout.flush()
}
