use crate::{
    custom_buffers::files_buffer::FilesBuffer,
    util::{Command, open_file},
};

impl FilesBuffer {
    fn create_command(&mut self, args: &str) -> Command {
        // Create only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::create_dir_all(args) {
                return Command::Error(err.to_string());
            }
        // `open_file` creates the directory hierarchy and file.
        } else if let Err(err) = open_file(args) {
            return Command::Error(err.to_string());
        }

        self.refresh()
    }

    pub(super) fn remove_command(&mut self, args: &str) -> Command {
        // Remove only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::remove_dir(args) {
                return Command::Error(err.to_string());
            }
        } else if let Err(err) = std::fs::remove_file(args) {
            return Command::Error(err.to_string());
        }

        self.refresh()
    }

    pub(super) fn recursive_remove_command(&mut self, args: &str) -> Command {
        // Remove only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::remove_dir_all(args) {
                return Command::Error(err.to_string());
            }

            return self.refresh();
        }

        Command::Info("Recursive removal only works for directories".to_string())
    }

    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self, input: &str) -> Command {
        if input.is_empty() {
            return Command::Ok;
        }

        let (cmd, args) = match input.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (input.trim(), ""),
        };

        match cmd {
            "mk" => self.create_command(args),
            "rm" => self.remove_command(args),
            "rm!" => self.recursive_remove_command(args),
            _ => Command::Error(format!("Unrecognized command: '{cmd}'")),
        }
    }
}
