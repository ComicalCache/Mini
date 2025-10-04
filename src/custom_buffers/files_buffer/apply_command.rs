use crate::{
    INFO_BUFF_IDX, INFO_MSG, cursor, custom_buffers::files_buffer::FilesBuffer, util::CommandResult,
};
use std::borrow::Cow;

impl FilesBuffer {
    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self) -> CommandResult {
        if self.base.cmd.buff[0].is_empty() {
            return CommandResult::Ok;
        }

        let cmd_buff = self.base.cmd.buff[0].clone();
        let (cmd, _) = match cmd_buff.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd, args),
            None => (cmd_buff.as_str(), ""),
        };

        match cmd {
            "q" => CommandResult::Quit,
            "qq" => CommandResult::ForceQuit,
            "?" => CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                INFO_MSG.lines().map(Cow::from).collect(),
                None,
            ),
            _ => {
                if let Ok(dest) = cmd.parse::<usize>() {
                    cursor::jump_to_line(&mut self.base.doc, &mut self.base.view, dest);
                    return CommandResult::Ok;
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
