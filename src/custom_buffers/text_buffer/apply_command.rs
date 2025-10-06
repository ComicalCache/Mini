use crate::{
    INFO_BUFF_IDX,
    custom_buffers::text_buffer::TextBuffer,
    util::{CommandResult, open_file, read_file_to_lines},
};
use std::{borrow::Cow, io::Error};

impl TextBuffer {
    fn write_to_file(&mut self) -> Result<bool, Error> {
        let Some(file) = self.file.as_mut() else {
            return Ok(false);
        };

        self.base.doc.write_to_file(file)?;

        Ok(true)
    }

    fn open_command(&mut self, args: &str, force: bool) -> CommandResult {
        if self.base.doc.edited && !force {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(
                    "There are unsaved changes, save or oo to force open new",
                )],
                None,
            );
        }

        // Reset state.
        self.base.doc.clear(0, 0);
        self.base.cmd.clear(0, 0);
        self.base
            .view
            .init(self.base.view.w, self.base.view.h, 0, 0, 1);
        self.file = None;

        // Open blank buffer if no path is specified.
        if args.is_empty() {
            return CommandResult::Ok;
        }

        self.file = match open_file(args) {
            Ok(file) => Some(file),
            Err(err) => {
                return CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(err.to_string())],
                    None,
                );
            }
        };

        match read_file_to_lines(self.file.as_mut().unwrap()) {
            Ok(lines) => self.base.doc.set_contents(&lines, 0, 0),
            Err(err) => {
                return CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(err.to_string())],
                    None,
                );
            }
        }

        CommandResult::Ok
    }

    fn write_command(&mut self, args: &str) -> CommandResult {
        if !args.is_empty() {
            self.file = match open_file(args) {
                Ok(file) => Some(file),
                Err(err) => {
                    return CommandResult::SetAndChangeBuffer(
                        INFO_BUFF_IDX,
                        vec![Cow::from(err.to_string())],
                        None,
                    );
                }
            };
        }

        // Failed to write file.
        let res = match self.write_to_file() {
            Ok(res) => res,
            Err(err) => {
                return CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(err.to_string())],
                    None,
                );
            }
        };
        if !res {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(
                    "Please specify a file location using 'w <path>' to write the file to",
                )],
                None,
            );
        }

        CommandResult::Ok
    }

    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self, cmd: &str) -> CommandResult {
        if cmd.is_empty() {
            return CommandResult::Ok;
        }

        let (cmd, args) = match cmd.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (cmd.trim(), ""),
        };

        match cmd {
            "wq" => match self.write_to_file() {
                Ok(res) if !res => CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(
                        "Please specify a file location using 'w <path>' to write the file to",
                    )],
                    None,
                ),
                Err(err) => CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(err.to_string())],
                    None,
                ),
                _ => CommandResult::Quit,
            },
            "w" => self.write_command(args),
            "o" => self.open_command(args, false),
            "oo" => self.open_command(args, true),
            _ => CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(format!("Unrecognized command: '{cmd}'"))],
                None,
            ),
        }
    }
}
