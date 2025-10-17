use crate::{
    INFO_BUFF_IDX,
    buffer::history::{Change, Replace},
    cursor,
    custom_buffers::text_buffer::TextBuffer,
    util::{CommandResult, open_file, read_file_to_lines},
};
use regex::Regex;
use std::{borrow::Cow, io::Error};

impl TextBuffer {
    fn write_to_file(&mut self) -> Result<bool, Error> {
        let Some(file) = self.file.as_mut() else {
            return Ok(false);
        };

        self.base.doc.write_to_file(file)?;

        Ok(true)
    }

    fn open_command(&mut self, args: &str, force: bool) -> CommandResult {
        if !force && self.base.doc.edited {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(
                    "There are unsaved changes, save or oo to force open new",
                )],
                None,
            );
        }

        // Reset state.
        self.base.doc.clear(0, 0);
        self.base.cmd.clear(0, 0);
        self.base
            .doc_view
            .init(self.base.doc_view.w, self.base.doc_view.h, 0, 0, Some(1));
        self.file = None;

        // Open blank buffer if no path is specified.
        if args.is_empty() {
            return CommandResult::Ok;
        }

        self.file = match open_file(args) {
            Ok(file) => Some(file),
            Err(err) => {
                return CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(err.to_string())],
                    None,
                );
            }
        };

        match read_file_to_lines(self.file.as_mut().unwrap()) {
            Ok(lines) => self.base.doc.set_contents(&lines, 0, 0),
            Err(err) => {
                return CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(err.to_string())],
                    None,
                );
            }
        }

        CommandResult::Ok
    }

    fn write_command(&mut self, args: &str) -> CommandResult {
        if !args.is_empty() {
            self.file = match open_file(args) {
                Ok(file) => Some(file),
                Err(err) => {
                    return CommandResult::SetAndChangeBuffer(
                        INFO_BUFF_IDX,
                        vec![Cow::from(err.to_string())],
                        None,
                    );
                }
            };
        }

        // Failed to write file.
        let res = match self.write_to_file() {
            Ok(res) => res,
            Err(err) => {
                return CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(err.to_string())],
                    None,
                );
            }
        };
        if !res {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(
                    "Please specify a file location using 'w <path>' to write the file to",
                )],
                None,
            );
        }

        CommandResult::Ok
    }

    fn replace_command(&mut self, args: &str) -> CommandResult {
        let Some(end) = self.base.sel else {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from("Replace command requires a selection.")],
                None,
            );
        };

        let err = CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from("Invalid format. Expected: r /<regex>/<replace>/")],
            None,
        );
        let Some(args) = args.strip_prefix('/') else {
            return err;
        };
        let Some((regex_str, replace_str)) = args.split_once('/') else {
            return err;
        };
        let Some(replace_str) = replace_str.strip_suffix('/') else {
            return err;
        };
        if regex_str.is_empty() {
            return err;
        }

        let regex = match Regex::new(regex_str) {
            Ok(regex) => regex,
            Err(err) => {
                let mut buff = vec![Cow::from(format!(
                    "'{regex_str}' is not a valid regular expression:",
                ))];
                buff.extend(err.to_string().lines().map(str::to_string).map(Cow::from));
                return CommandResult::SetAndChangeBuffer(INFO_BUFF_IDX, buff, None);
            }
        };

        let start = self.base.doc.cur;
        let (start, end) = if start < end {
            (start, end)
        } else {
            (end, start)
        };

        let hay = self.base.doc.get_range(start, end).expect("Illegal state");

        let mut new = String::new();
        let mut last_match = 0;
        let mut changes = Vec::new();
        for captures in regex.captures_iter(&hay) {
            // Fetch text between matches.
            let mat = captures.get(0).unwrap();
            new.push_str(&hay[last_match..mat.start()]);

            // Save pos of replacement in new string.
            let pos = cursor::end_pos(&start, &new);

            // Replace match.
            let mut replacement = String::new();
            captures.expand(replace_str, &mut replacement);
            new.push_str(&replacement);

            // Add replace operation to history.
            let delete_data = Cow::from(mat.as_str().to_string());
            let insert_data = Cow::from(replacement);
            changes.push(Replace {
                pos,
                delete_data,
                insert_data,
            });

            last_match = mat.end();
        }
        new.push_str(&hay[last_match..]);

        // Replace buffer content.
        self.base.doc.remove_range(start, end);
        self.base.doc.write_str_at(start.x, start.y, &new);
        self.base.sel = None;

        if !changes.is_empty() {
            self.history.add_change(Change::Replace(changes));
        }

        CommandResult::Ok
    }

    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self, cmd: &str) -> CommandResult {
        if cmd.is_empty() {
            return CommandResult::Ok;
        }

        let (cmd, args) = match cmd.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (cmd.trim(), ""),
        };

        match cmd {
            "wq" => match self.write_to_file() {
                Ok(res) if !res => CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(
                        "Please specify a file location using 'w <path>' to write the file to",
                    )],
                    None,
                ),
                Err(err) => CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    vec![Cow::from(err.to_string())],
                    None,
                ),
                _ => CommandResult::Quit,
            },
            "w" => self.write_command(args),
            "o" => self.open_command(args, false),
            "oo" => self.open_command(args, true),
            "r" => self.replace_command(args),
            _ => CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(format!("Unrecognized command: '{cmd}'"))],
                None,
            ),
        }
    }
}
