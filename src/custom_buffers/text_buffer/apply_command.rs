use crate::{
    INFO_BUFF_IDX, INFO_MSG, cursor,
    custom_buffers::text_buffer::TextBuffer,
    util::{CommandResult, open_file, read_file_to_lines},
};
use std::{borrow::Cow, io::Error};

impl TextBuffer {
    fn write_to_file(&mut self) -> Result<bool, Error> {
        let Some(file) = self.file.as_mut() else {
            return Ok(false);
        };

        self.doc.write_to_file(file)?;

        Ok(true)
    }

    fn quit_command(&mut self) -> CommandResult {
        if !self.doc.edited {
            return CommandResult::Quit;
        }

        CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(
                "There are unsaved changes, save or qq to force quit",
            )],
            None,
        )
    }

    fn open_command(&mut self, args: &str, force: bool) -> CommandResult {
        if self.doc.edited && !force {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(
                    "There are unsaved changes, save or oo to force open new",
                )],
                None,
            );
        }

        // Reset state.
        self.doc.clear(0, 0);
        self.cmd.clear(0, 0);
        self.view.init(self.view.w, self.view.h, 0, 0, 1);
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
            Ok(lines) => self.doc.set_contents(&lines, 0, 0),
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

    fn jump_to_line(&mut self, mut dest: usize) -> CommandResult {
        // At most the len of the buffer, at least 1, then subtract one to get the correct index.
        dest = dest.min(self.doc.buff.len()).max(1) - 1;

        let y = self.doc.cur.y;
        if dest < y {
            cursor::up(&mut self.doc, &mut self.view, y - dest);
        } else if dest > y {
            cursor::down(&mut self.doc, &mut self.view, dest - y);
        }

        CommandResult::Ok
    }

    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self) -> CommandResult {
        if self.cmd.buff[0].is_empty() {
            return CommandResult::Ok;
        }

        let cmd_buff = self.cmd.buff[0].clone();
        let (cmd, args) = match cmd_buff.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd, args),
            None => (cmd_buff.as_str(), ""),
        };

        match cmd {
            "q" => self.quit_command(),
            "qq" => CommandResult::ForceQuit,
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
            "?" => CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                INFO_MSG.lines().map(Cow::from).collect(),
                None,
            ),
            _ => {
                if let Ok(dest) = cmd.parse::<usize>() {
                    return self.jump_to_line(dest);
                }

                CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(format!("Unrecognized command: '{cmd}'"))],
                    None,
                )
            }
        }
    }
}
