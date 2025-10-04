use crate::{
    INFO_BUFF_IDX, INFO_MSG, buffer::Buffer, cursor, custom_buffers::info_buffer::InfoBuffer,
    util::CommandResult,
};
use std::borrow::Cow;

impl InfoBuffer {
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
            "clear" => {
                self.set_contents(&[Cow::from("")], None);
                CommandResult::Ok
            }
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
