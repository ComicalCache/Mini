use crate::{
    buffer::BufferResult,
    cursor::{self, Cursor},
    custom_buffers::text_buffer::TextBuffer,
    history::Replace,
    selection::{Selection, SelectionKind},
    shell_command::ShellCommand,
    util::{file_name, open_file},
};
use regex::Regex;
use std::io::{Error, Read};

impl TextBuffer {
    fn write_to_file(&mut self) -> Result<bool, Error> {
        let Some(file) = self.file.as_mut() else {
            return Ok(false);
        };

        self.base.doc.write_to_file(file)?;
        Ok(true)
    }

    fn open_command(&mut self, args: &str, force: bool) -> BufferResult {
        if !force && self.base.doc.edited {
            return BufferResult::Error(
                "There are unsaved changes, save or oo to force open a new document".to_string(),
            );
        }

        // Reset state.
        self.base.doc.from("");
        self.base.cmd.from("");
        self.base.doc_view.cur = Cursor::new(0, 0);
        self.base.doc_view.set_gutter_width(1);
        self.file = None;
        self.file_name = None;

        // Open blank buffer if no path is specified.
        if args.is_empty() {
            return BufferResult::Ok;
        }

        self.file = match open_file(args) {
            Ok(file) => Some(file),
            Err(err) => {
                return BufferResult::Error(err.to_string());
            }
        };
        self.file_name = file_name(args);

        let mut buff = String::new();
        match self.file.as_mut().unwrap().read_to_string(&mut buff) {
            Ok(_) => self.base.doc.from(buff.as_str()),
            Err(err) => {
                return BufferResult::Error(err.to_string());
            }
        }

        BufferResult::Ok
    }

    fn write_command(&mut self, args: &str) -> BufferResult {
        if !args.is_empty() {
            self.file = match open_file(args) {
                Ok(file) => Some(file),
                Err(err) => {
                    return BufferResult::Error(err.to_string());
                }
            };
            self.file_name = file_name(args);
        }

        let res = match self.write_to_file() {
            Ok(res) => res,
            Err(err) => {
                return BufferResult::Error(err.to_string());
            }
        };
        // Failed to write file because no path exists.
        if !res {
            return BufferResult::Error(
                "Please specify a file location using 'w <path>' to write the file to".to_string(),
            );
        }

        BufferResult::Info(format!(
            "File has been written to {}",
            self.file_name.as_ref().unwrap()
        ))
    }

    fn replace_command(&mut self, args: &str) -> BufferResult {
        let err =
            BufferResult::Error("Invalid format. Expected: r /<regex>/<replace>/".to_string());
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
                return BufferResult::Error(format!(
                    "'{regex_str}' is not a valid regular expression:\n{err}"
                ));
            }
        };

        // Use selections or replace in entire buffer.
        self.base.selections.sort_unstable();
        let selections = if self.base.selections.is_empty() {
            // Save previous cursor position.
            let tmp_view_cur = self.base.doc_view.cur;
            let tmp_doc_cur = self.base.doc.cur;

            let start = Cursor::new(0, 0);
            cursor::jump_to_end_of_file(&mut self.base.doc, &mut self.base.doc_view);
            let end = self.base.doc.cur;

            // Restore previous cursor position.
            self.base.doc.cur = tmp_doc_cur;
            self.base.doc_view.cur = tmp_view_cur;

            &[Selection::new(
                start,
                end,
                SelectionKind::Normal,
                None,
                None,
            )]
        } else {
            &self.base.selections[..]
        };

        let mut changes = Vec::new();
        for selection in selections {
            let (start, end) = selection.range();

            let hay = self.base.doc.get_range(start, end).unwrap().to_string();

            let mut new = String::new();
            let mut last_match = 0;
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
                let delete_data = mat.as_str().to_string();
                let insert_data = replacement;
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
        }

        self.base.clear_selections();

        if !changes.is_empty() {
            self.history.add_change(changes);
        }

        BufferResult::Ok
    }

    fn execute_shell_command(&mut self, args: &str) -> BufferResult {
        self.shell_command = match ShellCommand::new(
            self.base.doc_view.buff_w,
            self.base.doc_view.h,
            args.to_string(),
        ) {
            Ok(sc) => Some(sc),
            Err(err) => return err,
        };
        BufferResult::Ok
    }

    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self, cmd: &str) -> BufferResult {
        if cmd.is_empty() {
            return BufferResult::Ok;
        }

        let (cmd, args) = match cmd.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (cmd.trim(), ""),
        };

        match cmd {
            "wq" => match self.write_to_file() {
                Ok(res) if !res => BufferResult::Error(
                    "Please specify a file location using 'w <path>' to write the file to"
                        .to_string(),
                ),
                Err(err) => BufferResult::Error(err.to_string()),
                _ => BufferResult::Quit,
            },
            "w" => self.write_command(args),
            "o" => self.open_command(args, false),
            "oo" => self.open_command(args, true),
            "r" => self.replace_command(args),
            "cmd" => self.execute_shell_command(args),
            _ => BufferResult::Error(format!("Unrecognized command: '{cmd}'")),
        }
    }
}
