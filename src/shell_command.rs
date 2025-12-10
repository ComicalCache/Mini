use crate::{
    buffer::BufferResult,
    util::{application_key_to_string, key_to_string},
};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::{
    io::{Error, Read, Write},
    sync::mpsc::{self, Receiver},
    thread,
};
use termion::event::Key;
use vt100::Parser;

const SCROLLBACK_LEN: usize = 5000;

pub enum ShellCommandResult {
    Data(Vec<u8>),
    Error(String),
    Eof,
}

/// A helper to run shell commands in the background and stream the output.
pub struct ShellCommand {
    /// The command to run.
    pub cmd: String,

    /// The command output stream.
    pub rx: Receiver<ShellCommandResult>,

    /// Writer to the shell command.
    writer: Box<dyn Write + Send>,

    /// The VT100 parser maintaining the terminal state.
    pub parser: Parser,
}

impl ShellCommand {
    pub fn new(w: usize, h: usize, cmd: String) -> Result<Self, BufferResult> {
        use ShellCommandResult::{Eof, Error};

        // Create a pseudo terminal.
        let pty = native_pty_system();
        // The dimensions are reported by the terminal.
        #[allow(clippy::cast_possible_truncation)]
        let pair = match pty.openpty(PtySize {
            rows: h as u16,
            cols: w as u16,
            ..Default::default()
        }) {
            Ok(pair) => pair,
            Err(err) => {
                return Err(BufferResult::Error(err.to_string()));
            }
        };

        // Create the command to run in the pseudo terminal.
        let mut cb = CommandBuilder::new("fish");
        cb.arg("-c");
        cb.arg(cmd.clone());
        if let Ok(cwd) = std::env::current_dir() {
            cb.cwd(cwd);
        }
        let mut child = match pair.slave.spawn_command(cb) {
            Ok(child) => child,
            Err(err) => return Err(BufferResult::Error(err.to_string())),
        };

        // Get the reader and writer to interface with the command in the pseudo terminal.
        let mut reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(err) => {
                return Err(BufferResult::Error(err.to_string()));
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(err) => {
                return Err(BufferResult::Error(err.to_string()));
            }
        };

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buff = [0u8; 2048];

            loop {
                match reader.read(&mut buff) {
                    // EOF reached.
                    Ok(0) => break,
                    Ok(n) => {
                        // Send raw bytes to the main thread.
                        if tx
                            .send(ShellCommandResult::Data(buff[..n].to_vec()))
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(err) => {
                        let _ = tx.send(Error(err.to_string()));
                        return;
                    }
                }
            }

            if let Err(err) = child.wait() {
                let _ = tx.send(Error(err.to_string()));
                return;
            }

            let _ = tx.send(Eof);
        });

        // The indices are bound by terminal dimensions.
        #[allow(clippy::cast_possible_truncation)]
        let parser = Parser::new(h as u16, w as u16, SCROLLBACK_LEN);
        Ok(Self {
            cmd,
            rx,
            writer,
            parser,
        })
    }

    /// Write data to the command.
    pub fn write(&mut self, key: Key) -> Result<(), Error> {
        let data = if self.parser.screen().application_cursor() {
            application_key_to_string(key).or_else(|| key_to_string(key))
        } else {
            key_to_string(key)
        };
        let Some(data) = data else {
            return Ok(());
        };

        self.writer.write_all(data.as_bytes())?;
        self.writer.flush()
    }

    /// Get all data of the command.
    pub fn contents(&mut self) -> String {
        let screen = self.parser.screen_mut();
        let rows = screen.size().0 as usize;
        let mut contents = String::new();

        // Find the length of the scrollback.
        screen.set_scrollback(SCROLLBACK_LEN);
        let mut curr = screen.scrollback();

        while curr > 0 {
            screen.set_scrollback(curr);
            let page = screen.contents();

            // Append only lines not yet taken.
            let next = curr.saturating_sub(rows);
            for line in page.lines().take(curr - next) {
                contents.push_str(line);
                contents.push('\n');
            }

            curr = next;
        }

        // Append the latest terminal screen.
        screen.set_scrollback(0);
        contents.push_str(&screen.contents());

        contents
    }
}
