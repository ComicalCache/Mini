use crate::{buffer::BufferResult, buffer_impls::files_buffer::FilesBuffer, util::open_file};

impl FilesBuffer {
    fn create_command(&mut self, args: &str) -> BufferResult {
        // Create only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::create_dir_all(args) {
                return BufferResult::Error(err.to_string());
            }
        // `open_file` creates the directory hierarchy and file.
        } else if let Err(err) = open_file(args) {
            return BufferResult::Error(err.to_string());
        }

        self.refresh()
    }

    pub(super) fn remove_command(&mut self, args: &str) -> BufferResult {
        // Remove only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::remove_dir(args) {
                return BufferResult::Error(err.to_string());
            }
        } else if let Err(err) = std::fs::remove_file(args) {
            return BufferResult::Error(err.to_string());
        }

        self.refresh()
    }

    pub(super) fn recursive_remove_command(&mut self, args: &str) -> BufferResult {
        // Remove only directories.
        if args.ends_with('/') {
            if let Err(err) = std::fs::remove_dir_all(args) {
                return BufferResult::Error(err.to_string());
            }

            return self.refresh();
        }

        BufferResult::Info("Recursive removal only works for directories".to_string())
    }

    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self, input: &str) -> BufferResult {
        if input.is_empty() {
            return BufferResult::Ok;
        }

        let (cmd, args) = match input.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (input.trim(), ""),
        };

        match cmd {
            "mk" => self.create_command(args),
            "rm" => self.remove_command(args),
            "rm!" => self.recursive_remove_command(args),
            _ => BufferResult::Error(format!("Unrecognized command: '{cmd}'")),
        }
    }
}
