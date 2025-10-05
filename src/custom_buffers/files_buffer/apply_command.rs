use crate::{
    INFO_BUFF_IDX, INFO_MSG, cursor,
    custom_buffers::files_buffer::FilesBuffer,
    util::{CommandResult, line_column},
};
use std::borrow::Cow;

impl FilesBuffer {
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
            "q" => CommandResult::Quit,
            "qq" => CommandResult::ForceQuit,
            "?" => CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                INFO_MSG.lines().map(Cow::from).collect(),
                None,
            ),
            "goto" => {
                let (x, y) = line_column(args);
                cursor::jump_to_line_and_column(&mut self.base.doc, &mut self.base.view, x, y);

                CommandResult::Ok
            }
            _ => CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(format!("Unrecognized command: '{cmd}'"))],
                None,
            ),
        }
    }
}
