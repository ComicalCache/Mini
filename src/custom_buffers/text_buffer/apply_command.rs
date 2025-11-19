use crate::{
    INFO_BUFF_IDX,
    cursor::{self, Cursor},
    custom_buffers::text_buffer::TextBuffer,
    history::{Change, Replace},
    sc_buff,
    util::{CommandResult, file_name, open_file},
};
use regex::Regex;
use std::{
    borrow::Cow,
    io::{Error, Read},
};

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
            return sc_buff!(
                self,
                INFO_BUFF_IDX,
                "There are unsaved changes, save or oo to force open a new document".to_string(),
            );
        }

        // Reset state.
        self.base.doc.clear();
        self.base.cmd.clear();
        self.base.doc_view.cur = Cursor::new(0, 0);
        self.base.doc_view.set_gutter_width(1);
        self.file = None;
        self.file_name = None;

        // Open blank buffer if no path is specified.
        if args.is_empty() {
            return CommandResult::Ok;
        }

        self.file = match open_file(args) {
            Ok(file) => Some(file),
            Err(err) => return sc_buff!(self, INFO_BUFF_IDX, err.to_string()),
        };
        self.file_name = file_name(args);

        let mut buff = String::new();
        match self.file.as_mut().unwrap().read_to_string(&mut buff) {
            Ok(_) => {
                self.base.doc.set_contents(buff);
                self.base
                    .doc
                    .highlighter
                    .set_lang_filename(self.file_name.as_ref().unwrap());
            }
            Err(err) => return sc_buff!(self, INFO_BUFF_IDX, err.to_string()),
        }

        CommandResult::Ok
    }

    fn write_command(&mut self, args: &str) -> CommandResult {
        if !args.is_empty() {
            self.file = match open_file(args) {
                Ok(file) => Some(file),
                Err(err) => return sc_buff!(self, INFO_BUFF_IDX, err.to_string()),
            };
            self.file_name = file_name(args);
        }

        let res = match self.write_to_file() {
            Ok(res) => res,
            Err(err) => return sc_buff!(self, INFO_BUFF_IDX, err.to_string()),
        };
        // Failed to write file because no path exists.
        if !res {
            return sc_buff!(
                self,
                INFO_BUFF_IDX,
                "Please specify a file location using 'w <path>' to write the file to".to_string(),
            );
        }

        CommandResult::Ok
    }

    fn replace_command(&mut self, args: &str) -> CommandResult {
        let err = sc_buff!(
            self,
            INFO_BUFF_IDX,
            "Invalid format. Expected: r /<regex>/<replace>/".to_string(),
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
                return sc_buff!(
                    self,
                    INFO_BUFF_IDX,
                    format!("'{regex_str}' is not a valid regular expression:\n{err}"),
                );
            }
        };

        // Use selection or replace in entire buffer.
        let (start, end) = if let Some(end) = self.base.sel {
            if self.base.doc.cur < end {
                (self.base.doc.cur, end)
            } else {
                (end, self.base.doc.cur)
            }
        } else {
            // Save previous cursor position.
            let tmp_view_cur = self.base.doc_view.cur;
            let tmp_doc_cur = self.base.doc.cur;

            let start = Cursor::new(0, 0);
            cursor::jump_to_end_of_file(&mut self.base.doc, &mut self.base.doc_view);
            let end = self.base.doc.cur;

            // Restore previous cursor position.
            self.base.doc.cur = tmp_doc_cur;
            self.base.doc_view.cur = tmp_view_cur;

            (start, end)
        };

        let hay = self.base.doc.get_range(start, end).unwrap();

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

    fn syntax_command(&mut self, args: &str) -> CommandResult {
        if !self.base.doc.highlighter.set_lang(args) {
            return sc_buff!(self, INFO_BUFF_IDX, "Invalid language selected".to_string());
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
            "q" => CommandResult::Quit,
            "qq" => CommandResult::ForceQuit,
            "wq" => match self.write_to_file() {
                Ok(res) if !res => sc_buff!(
                    self,
                    INFO_BUFF_IDX,
                    "Please specify a file location using 'w <path>' to write the file to"
                        .to_string(),
                ),
                Err(err) => sc_buff!(self, INFO_BUFF_IDX, err.to_string()),
                _ => CommandResult::Quit,
            },
            "w" => self.write_command(args),
            "o" => self.open_command(args, false),
            "oo" => self.open_command(args, true),
            "r" => self.replace_command(args),
            "syntax" => self.syntax_command(args),
            _ => sc_buff!(
                self,
                INFO_BUFF_IDX,
                format!("Unrecognized command: '{cmd}'"),
            ),
        }
    }
}
