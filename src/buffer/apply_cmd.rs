use crate::{
    INFO_MSG,
    buffer::Buffer,
    util::{CmdResult, read_file},
};
use std::{
    fs::OpenOptions,
    io::{BufWriter, Error, Seek, SeekFrom, Write},
};

impl Buffer {
    fn write_to_file(&mut self) -> Result<bool, Error> {
        if !self.edited {
            return Ok(true);
        }

        let Some(file) = self.file.as_mut() else {
            return Ok(false);
        };

        let size: u64 = self.line_buff.iter().map(|s| s.len() as u64 + 1).sum();
        file.set_len(size.saturating_sub(1))?;

        file.seek(SeekFrom::Start(0))?;
        let mut writer = BufWriter::new(file);
        for line in &self.line_buff {
            writeln!(writer, "{line}")?;
        }
        writer.flush()?;

        self.edited = false;
        Ok(true)
    }

    fn quit_cmd(&mut self) -> CmdResult {
        if !self.edited {
            return CmdResult::Quit;
        }

        self.cmd_pos.x = self.cmd_buff.chars().count();
        self.term_cmd_pos.x = self.cmd_buff.chars().count() + 1;

        CmdResult::Info("There are unsaved changes, save or qq to force quit".to_string())
    }

    fn open_cmd(&mut self, args: &str, force: bool) -> CmdResult {
        if self.edited && !force {
            return CmdResult::Info(
                "There are unsaved changes, save or oo to force open new".to_string(),
            );
        }

        self.reinit();

        // Open blank buffer if no path is specified
        if args.is_empty() {
            return CmdResult::Continue;
        }

        self.file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(args)
        {
            Ok(file) => Some(file),
            Err(err) => return CmdResult::Info(err.to_string()),
        };

        let line_buff = match read_file(self.file.as_mut().unwrap()) {
            Ok(line_buff) => line_buff,
            Err(err) => return CmdResult::Info(err.to_string()),
        };
        if !line_buff.is_empty() {
            self.line_buff.resize(line_buff.len(), String::new());
            for (idx, line) in line_buff.iter().enumerate() {
                self.line_buff[idx].clone_from(line);
            }
        }

        CmdResult::Continue
    }

    fn write_cmd(&mut self, args: &str) -> CmdResult {
        if !args.is_empty() {
            self.file = match OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(args)
            {
                Ok(file) => Some(file),
                Err(err) => return CmdResult::Info(err.to_string()),
            };
        }

        // Failed to write file
        let res = match self.write_to_file() {
            Ok(res) => res,
            Err(err) => return CmdResult::Info(err.to_string()),
        };
        if !res {
            return CmdResult::Info(
                "Please specify a file location using 'w <path>' to write the file to".to_string(),
            );
        }

        CmdResult::Continue
    }

    /// Applies the command entered during command mode
    pub fn apply_cmd(&mut self) -> CmdResult {
        let cmd_buff = self.cmd_buff.clone();
        let (cmd, args) = match cmd_buff.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd, args),
            None => (cmd_buff.as_str(), ""),
        };

        match cmd {
            "q" => self.quit_cmd(),
            "qq" => CmdResult::Quit,
            "wq" => {
                let res = match self.write_to_file() {
                    Ok(res) => res,
                    Err(err) => return CmdResult::Info(err.to_string()),
                };
                if !res {
                    return CmdResult::Info(
                        "Please specify a file location using 'w <path>' to write the file to"
                            .to_string(),
                    );
                }

                CmdResult::Quit
            }
            "w" => self.write_cmd(args),
            "o" => self.open_cmd(args, false),
            "oo" => self.open_cmd(args, true),
            "?" => CmdResult::Info(INFO_MSG.to_string()),
            _ => CmdResult::Info(format!("Unrecognized command: '{cmd}'")),
        }
    }
}
