use crate::{
    buffer::Buffer,
    util::{CmdResult, read_file},
};
use std::{fs::OpenOptions, io::Error};

impl Buffer {
    fn __quit(&mut self) -> CmdResult {
        if !self.edited {
            return CmdResult::Quit;
        }

        self.cmd_pos.x = self.cmd_buff.chars().count();
        self.term_cmd_pos.x = self.cmd_buff.chars().count() + 1;

        CmdResult::Error("There are unsafed changes, save or qq to force quit".to_string())
    }

    fn __open(&mut self, args: &str, force: bool) -> Result<CmdResult, Error> {
        if self.edited && !force {
            return Ok(CmdResult::Error(
                "There are unsafed changes, save or oo to force open new".to_string(),
            ));
        }

        self.reinit();

        // Open blank buffer if no path is specified
        if args.is_empty() {
            self.file = None;
            self.line_buff.clear();
            self.line_buff.push(String::new());
            return Ok(CmdResult::Continue);
        }

        self.file = Some(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(args)?,
        );

        self.line_buff.clear();
        self.line_buff = read_file(self.file.as_mut().unwrap())?;
        if self.line_buff.is_empty() {
            self.line_buff.push(String::new());
        }

        Ok(CmdResult::Continue)
    }

    fn __write(&mut self, args: &str) -> Result<CmdResult, Error> {
        if !args.is_empty() {
            self.file = Some(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(args)?,
            );
        }

        // Failed to write file
        if !self.write_to_file()? {
            return Ok(CmdResult::Error(
                "Please specify a file location using 'w <path>' to write the file to".to_string(),
            ));
        }

        Ok(CmdResult::Continue)
    }

    /// Applies the command entered during command mode
    pub fn apply_cmd(&mut self) -> Result<CmdResult, Error> {
        let cmd_buff = self.cmd_buff.clone();
        let (cmd, args) = match cmd_buff.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd, args),
            None => (cmd_buff.as_str(), ""),
        };

        match cmd {
            "q" => Ok(self.__quit()),
            "qq" => Ok(CmdResult::Quit),
            "wq" => {
                // Failed to write file
                if !self.write_to_file()? {
                    return Ok(CmdResult::Error(
                        "Please specify a file location using 'w <path>' to write the file to"
                            .to_string(),
                    ));
                }

                Ok(CmdResult::Quit)
            }
            "w" => self.__write(args),
            "o" => self.__open(args, false),
            "oo" => self.__open(args, true),
            _ => Ok(CmdResult::Error(format!("Unrecognized command: '{cmd}'"))),
        }
    }
}
