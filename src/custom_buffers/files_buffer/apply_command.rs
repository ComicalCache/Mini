use crate::{
    INFO_BUFF_IDX, TXT_BUFF_IDX, c_buff,
    custom_buffers::files_buffer::FilesBuffer,
    sc_buff,
    util::{CommandResult, open_file},
};

impl FilesBuffer {
    fn create_command(&mut self, args: &str) -> CommandResult {
        // Create only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::create_dir_all(args) {
                return sc_buff!(self, INFO_BUFF_IDX, err.to_string());
            }
        // open_file creates the directory hierarchy and file.
        } else if let Err(err) = open_file(args) {
            return sc_buff!(self, INFO_BUFF_IDX, err.to_string());
        }

        self.refresh()
    }

    pub(super) fn remove_command(&mut self, args: &str) -> CommandResult {
        // Remove only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::remove_dir(args) {
                return sc_buff!(self, INFO_BUFF_IDX, err.to_string());
            }
        } else if let Err(err) = std::fs::remove_file(args) {
            return sc_buff!(self, INFO_BUFF_IDX, err.to_string());
        }

        self.refresh()
    }

    pub(super) fn recursive_remove_command(&mut self, args: &str) -> CommandResult {
        // Remove only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::remove_dir_all(args) {
                return sc_buff!(self, INFO_BUFF_IDX, err.to_string());
            }

            return self.refresh();
        }

        sc_buff!(
            self,
            INFO_BUFF_IDX,
            "Recursive removal only works for directories".to_string(),
        )
    }

    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self, input: &str) -> CommandResult {
        if input.is_empty() {
            return CommandResult::Ok;
        }

        let (cmd, args) = match input.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (input.trim(), ""),
        };

        match cmd {
            "q" => c_buff!(self, TXT_BUFF_IDX),
            "qq" => c_buff!(self, TXT_BUFF_IDX),
            "mk" => self.create_command(args),
            "rm" => self.remove_command(args),
            "rm!" => self.recursive_remove_command(args),
            _ => sc_buff!(
                self,
                INFO_BUFF_IDX,
                format!("Unrecognized command: '{cmd}'"),
            ),
        }
    }
}
