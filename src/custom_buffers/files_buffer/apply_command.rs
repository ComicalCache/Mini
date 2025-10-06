use crate::{INFO_BUFF_IDX, custom_buffers::files_buffer::FilesBuffer, util::CommandResult};
use std::borrow::Cow;

impl FilesBuffer {
    /// Applies the command entered during command mode.
    pub fn apply_command(input: &str) -> CommandResult {
        if input.is_empty() {
            return CommandResult::Ok;
        }

        let (cmd, _) = match input.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (input.trim(), ""),
        };

        CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(format!("Unrecognized command: '{cmd}'"))],
            None,
        )
    }
}
