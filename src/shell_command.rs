use duct::cmd;
use std::{
    io::{BufRead, BufReader},
    sync::mpsc::{self, Receiver},
    thread,
};

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
}

impl ShellCommand {
    pub fn new(cmd: String) -> Self {
        use ShellCommandResult::{Data, Eof, Error};

        let (tx, rx) = mpsc::channel();

        let thread_cmd = cmd.clone();
        thread::spawn(move || {
            if let Ok(reader) = cmd!("fish", "-c", thread_cmd).stderr_to_stdout().reader() {
                for line in BufReader::new(reader).lines() {
                    let res = match line {
                        Ok(data) => tx.send(Data(format!("{data}\n"))),
                        Err(err) => tx.send(Error(err.to_string())),
                    };
                    if res.is_err() {
                        return;
                    }
                }

                if tx.send(Eof).is_err() {
                    return;
                }
            }

            let _ = tx.send(Error("Failed to execute shell command".to_string()));
            let _ = tx.send(Eof);
        });

        Self { cmd, rx }
    }
}
