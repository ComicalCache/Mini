use crate::{
    INFO_BUFF_IDX, buffer::Buffer, custom_buffers::info_buffer::InfoBuffer, util::CommandResult,
};
use std::borrow::Cow;

impl InfoBuffer {
    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self, cmd: &str) -> CommandResult {
        if cmd.is_empty() {
            return CommandResult::Ok;
        }

        let (cmd, _) = match cmd.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (cmd.trim(), ""),
        };

        match cmd {
            "clear" => {
                self.set_contents(&[Cow::from("")], None);
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
