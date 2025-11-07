#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::too_many_lines, clippy::enum_glob_use, clippy::similar_names)]
#![feature(trait_alias)]

mod buffer;
mod cursor;
mod custom_buffers;
mod display;
mod document;
mod state_machine;
mod util;
mod viewport;

use crate::{
    buffer::Buffer,
    custom_buffers::{files_buffer::FilesBuffer, info_buffer::InfoBuffer, text_buffer::TextBuffer},
    display::Display,
    util::{CommandResult, file_name, open_file, split_to_lines},
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
const INFO_MSG: &str = include_str!("info.txt");

// Indices of buffers.
const TXT_BUFF_IDX: usize = 0;
const FILES_BUFF_IDX: usize = 1;
const INFO_BUFF_IDX: usize = 2;

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
    let mut info_buffer = Box::new(InfoBuffer::new(w as usize, h as usize)?);
    let files_buffer = Box::new(FilesBuffer::new(w as usize, h as usize, base)?);
    let mut curr_buff = if let Some(Err(err)) = &file {
        // Open the `FilesBuffer` if a directory was specified as argument.
        if err.kind() == ErrorKind::IsADirectory {
            FILES_BUFF_IDX
        } else {
            info_buffer.set_contents(&split_to_lines(err.to_string()), None, None);
            INFO_BUFF_IDX
        }
    } else {
        TXT_BUFF_IDX
    };

    // Vector of buffers.
    let mut buffs: [Box<dyn Buffer>; 3] = [
        // 1
        Box::new(TextBuffer::new(
            w as usize,
            h as usize,
            file.and_then(std::result::Result::ok),
            file_name,
        )?),
        // 2
        files_buffer,
        // 3
        info_buffer,
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
            buff.resize(w as usize, h as usize);
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
            CommandResult::ChangeBuffer(idx) => {
                rerender_changed_buff = true;
                curr_buff = idx;
            }
            // Set a buffer and change to it if the buffer has no pending changes.
            CommandResult::SetAndChangeBuffer(idx, contents, path, file_name) => {
                if let Err(err) = buffs[idx].can_quit() {
                    buffs[INFO_BUFF_IDX].set_contents(&err, path, file_name);
                    curr_buff = INFO_BUFF_IDX;
                } else {
                    buffs[idx].set_contents(&contents, path, file_name);
                    curr_buff = idx;
                }
            }
            // Quit the app if there are no unsaved changes left.
            CommandResult::Quit => {
                quit = true;

                for idx in 0..buffs.len() {
                    if let Err(err) = buffs[idx].can_quit() {
                        buffs[INFO_BUFF_IDX].set_contents(&err, None, None);
                        curr_buff = INFO_BUFF_IDX;

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

        #[cfg(feature = "syntax-highlighting")]
        buffs[curr_buff].highlight();

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
