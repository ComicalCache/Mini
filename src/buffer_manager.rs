use std::{
    fs::File,
    io::{BufWriter, Error, ErrorKind, Seek, SeekFrom, Write},
    path::PathBuf,
};

use termion::event::Key;

use crate::{
    buffer::{Buffer, BufferKind},
    custom_buffers::{files_buffer::FilesBuffer, text_buffer::TextBuffer},
    display::Display,
    message::{Message, MessageKind},
    util::{Command, open_file},
};

/// Manages open `Buffer`s and their interaction.
pub struct BufferManager {
    /// Total width of the `Display`.
    w: usize,
    /// Total height of the `Display`.
    h: usize,

    /// The dir where the program was launched at.
    base: PathBuf,

    /// Open `Buffer`.
    buffs: Vec<Box<dyn Buffer>>,
    /// Index of currently active `Buffer`.
    active: usize,
    /// Previously active `Buffer`.
    prev: Option<usize>,

    /// Log of messages to display on demand.
    log: Vec<Message>,

    /// Forces rerender after `Buffer` switching.
    force_rerender: bool,
}

impl BufferManager {
    pub fn new(
        path: Option<&String>,
        file: Option<Result<File, Error>>,
        file_name: Option<String>,
        w: usize,
        h: usize,
    ) -> Result<Self, Error> {
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

        let mut log = Vec::new();
        let buff: Box<dyn Buffer> = if let Some(Err(err)) = &file {
            if err.kind() == ErrorKind::IsADirectory {
                // Open the `FilesBuffer` if a directory was specified as argument.
                Box::new(FilesBuffer::new(w, h, 0, 0, base.clone())?)
            } else {
                // Show error in files buffer if failed to open.
                let mut files_buffer = Box::new(FilesBuffer::new(w, h, 0, 0, base.clone())?);
                files_buffer.set_message(MessageKind::Error, err.to_string());
                log.push(files_buffer.get_message().unwrap());
                files_buffer
            }
        } else {
            // Open the file if no error.
            Box::new(TextBuffer::new(
                w,
                h,
                0,
                0,
                file.and_then(Result::ok),
                file_name,
            )?)
        };

        Ok(Self {
            w,
            h,
            base,
            buffs: vec![buff],
            active: 0,
            prev: None,
            log,
            force_rerender: true,
        })
    }

    /// Handles the event, that the terminal was resized.
    pub fn resize(&mut self, w: usize, h: usize) {
        for buff in &mut self.buffs {
            buff.resize(w, h, 0, 0);
        }
    }

    /// Forwards a tick to the active `Buffer`.
    pub fn tick(&mut self, key: Option<Key>) -> bool {
        match self.buffs[self.active].tick(key) {
            Command::Ok => return true,
            Command::Change(idx) => {
                if idx >= self.buffs.len() {
                    let message = format!(
                        "No buffer at index `{idx}`.\n\
                        Use the `lb` command to list all open buffers or `nb <type>` to create a new buffer."
                    );
                    self.buffs[self.active].set_message(MessageKind::Error, message);
                    self.log
                        .push(self.buffs[self.active].get_message().unwrap());

                    return true;
                }

                self.prev = Some(self.active);
                self.active = idx;
                self.force_rerender = true;
            }
            Command::Info(message) => {
                self.buffs[self.active].set_message(MessageKind::Info, message);
                self.log
                    .push(self.buffs[self.active].get_message().unwrap());
            }
            Command::Error(message) => {
                self.buffs[self.active].set_message(MessageKind::Error, message);
                self.log
                    .push(self.buffs[self.active].get_message().unwrap());
            }
            Command::ListBuffers => {
                let mut message = String::new();
                for (idx, buff) in self.buffs.iter().enumerate() {
                    let mut tmp = if idx == self.active {
                        "[*] ".to_string()
                    } else {
                        format!("[{idx}] ")
                    };
                    match buff.kind() {
                        BufferKind::Text => {
                            tmp.push_str(format!("Text ({})\n", buff.name()).as_str());
                        }
                        BufferKind::Files => tmp.push_str("Files\n"),
                    }

                    message.push_str(tmp.as_str());
                }
                message.push_str("Use `cb <idx>` to switch to a buffer.");

                self.buffs[self.active].set_message(MessageKind::Info, message);
                self.log
                    .push(self.buffs[self.active].get_message().unwrap());
            }
            Command::NewBuffer(kind) => {
                self.prev = Some(self.active);
                self.active = self.buffs.len();

                match kind {
                    BufferKind::Text => self.buffs.push(Box::new(
                        TextBuffer::new(self.w, self.h, 0, 0, None, None).unwrap(),
                    )),
                    BufferKind::Files => self.buffs.push(Box::new(
                        FilesBuffer::new(self.w, self.h, 0, 0, self.base.clone()).unwrap(),
                    )),
                }
            }
            Command::Init(buff) => {
                self.buffs[self.active] = buff;
            }
            Command::Log => {
                // Create log file in the base directory.
                let mut log_file_path = self.base.clone();
                log_file_path.push("mini.log");

                if !self.write_log(&log_file_path) {
                    self.log
                        .push(self.buffs[self.active].get_message().unwrap());
                    return true;
                }

                // Show success message.
                self.log.clear();
                self.buffs[self.active].set_message(
                    MessageKind::Info,
                    format!("Log written to '{}'", log_file_path.to_string_lossy()),
                );
                self.log
                    .push(self.buffs[self.active].get_message().unwrap());
            }
            Command::Quit => {
                if let Err(err) = self.buffs[self.active].can_quit() {
                    self.buffs[self.active].set_message(MessageKind::Error, err);
                    self.log
                        .push(self.buffs[self.active].get_message().unwrap());
                    return true;
                }

                self.buffs.remove(self.active);

                // Quit the app if all buffers were closed.
                if self.buffs.is_empty() {
                    return false;
                }

                // Previous must exist if the list of buffers isn't empty.
                self.active = self.prev.unwrap_or(0).min(self.buffs.len() - 1);
                self.prev = None;
                self.force_rerender = true;
            }
            Command::ForceQuit => {
                self.buffs.remove(self.active);

                // Quit the app if all buffers were closed.
                if self.buffs.is_empty() {
                    return false;
                }

                // Previous must exist if the list of buffers isn't empty.
                self.active = self.prev.unwrap_or(0).min(self.buffs.len() - 1);
                self.prev = None;
                self.force_rerender = true;
            }
        }

        true
    }

    /// Renders the active `Buffer` to the `Display`.
    pub fn render(&mut self, display: &mut Display) {
        if self.force_rerender || self.buffs[self.active].need_rerender() {
            self.buffs[self.active].render(display);
        }

        self.force_rerender = false;
    }

    fn write_log(&mut self, log_file_path: &PathBuf) -> bool {
        let mut log_file = match open_file(log_file_path) {
            Ok(log_file) => BufWriter::new(log_file),
            Err(err) => {
                self.buffs[self.active].set_message(MessageKind::Error, err.to_string());
                return false;
            }
        };

        // Write log file.
        if let Err(err) = log_file.seek(SeekFrom::End(0)) {
            self.buffs[self.active].set_message(MessageKind::Error, err.to_string());
            return false;
        }
        for msg in &self.log {
            if let Err(err) = writeln!(&mut log_file, "{msg}") {
                self.buffs[self.active].set_message(MessageKind::Error, err.to_string());
                return false;
            }
        }
        if let Err(err) = log_file.flush() {
            self.buffs[self.active].set_message(MessageKind::Error, err.to_string());
            return false;
        }

        true
    }
}
