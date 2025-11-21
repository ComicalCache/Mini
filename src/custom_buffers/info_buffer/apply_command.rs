use crate::{
    TXT_BUFF_IDX, buffer::Buffer, cursor::Cursor, custom_buffers::info_buffer::InfoBuffer,
    util::CommandResult,
};

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
            "q" | "qq" => CommandResult::Change(TXT_BUFF_IDX),
            "clear" => {
                // Set contents moves the doc.cur to the beginning.
                self.set_contents(String::new(), None, None);
                self.base.doc_view.cur = Cursor::new(0, 0);
                CommandResult::Ok
            }
            _ => CommandResult::Info(format!("Unrecognized command: '{cmd}'")),
        }
    }
}
