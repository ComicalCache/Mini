use crate::{
    INFO_BUFF_IDX,
    custom_buffers::files_buffer::FilesBuffer,
    util::{CommandResult, open_file, split_to_lines},
};
use std::borrow::Cow;

impl FilesBuffer {
    fn create_command(&mut self, args: &str) -> CommandResult {
        // Create only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::create_dir_all(args) {
                return CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    split_to_lines(err.to_string()),
                    None,
                );
            }
        // open_file creates the directory hierarchy and file.
        } else if let Err(err) = open_file(args) {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                split_to_lines(err.to_string()),
                None,
            );
        }

        self.refresh()
    }

    pub(super) fn remove_command(&mut self, args: &str) -> CommandResult {
        // Remove only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::remove_dir(args) {
                return CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    split_to_lines(err.to_string()),
                    None,
                );
            }
        } else if let Err(err) = std::fs::remove_file(args) {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                split_to_lines(err.to_string()),
                None,
            );
        }

        self.refresh()
    }

    pub(super) fn recursive_remove_command(&mut self, args: &str) -> CommandResult {
        // Remove only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::remove_dir_all(args) {
                return CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    split_to_lines(err.to_string()),
                    None,
                );
            }

            return self.refresh();
        }

        CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from("Recursive removal only works for directories")],
            None,
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
            "mk" => self.create_command(args),
            "rm" => self.remove_command(args),
            "rm!" => self.recursive_remove_command(args),
            _ => CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(format!("Unrecognized command: '{cmd}'"))],
                None,
            ),
        }
    }
}
