mod performer;

use crate::{buffer::BufferResult, shell_command::performer::Performer, util::key_to_string};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::{
    io::{Error, Read, Write},
    sync::mpsc::{self, Receiver},
    thread,
};
use termion::event::Key;
use vte::Parser;

pub enum ShellCommandResult {
    Data(String),
    CarriageReturn,
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
            let mut parser = Parser::new();

            loop {
                match reader.read(&mut buff) {
                    // EOF reached.
                    Ok(0) => break,
                    Ok(n) => {
                        let mut performer = Performer::new();
                        parser.advance(
                            &mut performer,
                            String::from_utf8_lossy(&buff[..n])
                                .replace("\r\n", "\n")
                                .as_bytes(),
                        );

                        for item in performer.output {
                            if tx.send(item).is_err() {
                                return;
                            }
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
