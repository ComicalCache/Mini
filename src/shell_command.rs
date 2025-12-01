mod performer;
mod strip;
mod writer;

use crate::{buffer::BufferResult, shell_command::strip::strip_str, util::key_to_string};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::{
    io::{Error, Write},
    sync::mpsc::{self, Receiver},
    thread,
};
use termion::event::Key;

pub enum ShellCommandResult {
    Data(String),
    Error(String),
    Eof,
}

/// A helper to run shell commands in the background and stream the output.
pub struct ShellCommand {
    /// The command to run.
    pub cmd: String,

    /// The command output line by line.
    pub rx: Receiver<ShellCommandResult>,

    /// Writer to the shell command.
    writer: Box<dyn Write + Send>,
}

impl ShellCommand {
    pub fn new(w: usize, h: usize, cmd: String) -> Result<Self, BufferResult> {
        use ShellCommandResult::{Data, Eof, Error};

        // Create a pseudo terminal.
        let pty = native_pty_system();
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
        match std::env::current_dir() {
            Ok(cwd) => cb.cwd(cwd),
            _ => {}
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
                        let data = strip_str(String::from_utf8_lossy(&buff[..n]));
                        if tx.send(Data(data)).is_err() {
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

            if tx.send(Eof).is_err() {
                return;
            }
        });

        Ok(Self { cmd, rx, writer })
    }

    /// Write data to the command.
    pub fn write(&mut self, key: Key) -> Result<(), Error> {
        let Some(data) = key_to_string(key) else {
            return Ok(());
        };

        self.writer.write_all(data.as_bytes())?;
        self.writer.flush()
    }
}
