use crate::{
    INFO_BUFF_IDX, INFO_MSG,
    buffer::base::{BaseBuffer, CommandTick},
    cursor::{self, Cursor},
    sc_buff,
    util::{CommandResult, line_column, split_to_lines},
};
use regex::Regex;
use std::borrow::Cow;

impl<ModeEnum: Clone, ViewEnum: Clone, CommandEnum: Clone>
    BaseBuffer<ModeEnum, ViewEnum, CommandEnum>
{
    fn search(&mut self, args: &str) -> CommandResult {
        if args.len() == 2 || !args.starts_with('/') || !args.ends_with('/') {
            return sc_buff!(
                INFO_BUFF_IDX,
                ["Expected a valid regular expression like '/<regex>/'"],
                None,
            );
        }

        let regex = match Regex::new(&args[1..args.len() - 1]) {
            Ok(regex) => regex,
            Err(err) => {
                let mut buff = vec![Cow::from(format!(
                    "'{args}' is not a valid regular expression:",
                ))];
                buff.extend(split_to_lines(err.to_string()));
                return sc_buff!(INFO_BUFF_IDX, buff, None);
            }
        };

        let (start, end) = if let Some(end) = self.sel {
            (self.doc.cur, end)
        } else {
            // Save previous cursor position.
            let tmp_view_cur = self.doc_view.cur;
            let tmp_doc_cur = self.doc.cur;

            let start = Cursor::new(0, 0);
            cursor::jump_to_end_of_file(&mut self.doc, &mut self.doc_view);
            let end = self.doc.cur;

            // Restore previous cursor position.
            self.doc.cur = tmp_doc_cur;
            self.doc_view.cur = tmp_view_cur;

            (start, end)
        };
        let (start, end) = if start < end {
            (start, end)
        } else {
            (end, start)
        };

        let hay = self.doc.get_range(start, end).expect("Illegal state");
        self.matches = regex
            .find_iter(&hay)
            .map(|mat| {
                let start_pos = cursor::end_pos(&start, &hay[..mat.start()]);
                let end_pos = cursor::end_pos(&start, &hay[..mat.end()]);
                (start_pos, end_pos)
            })
            .collect();
        self.matches_idx = None;

        if self.matches.is_empty() {
            return sc_buff!(INFO_BUFF_IDX, ["No matches found"], None);
        }

        self.matches_idx = self
            .matches
            .iter()
            .enumerate()
            .find_map(|(idx, (start, _))| self.doc.cur.le(start).then_some(idx))
            // Or use last match if before current cursor position.
            .or(Some(self.matches.len() - 1));

        if let Some(idx) = self.matches_idx {
            self.sel = Some(self.matches[idx].1);
            cursor::move_to(&mut self.doc, &mut self.doc_view, self.matches[idx].0);
        }

        CommandResult::Ok
    }

    /// Applies the command entered during command mode.
    pub(super) fn apply_command(
        &mut self,
        input: Cow<'static, str>,
    ) -> Result<CommandResult, CommandTick<CommandEnum>> {
        if input.is_empty() {
            return Ok(CommandResult::Ok);
        }

        let (cmd, args) = match input.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (input.trim(), ""),
        };

        match cmd {
            "q" => Ok(CommandResult::Quit),
            "qq" => Ok(CommandResult::ForceQuit),
            "?" => {
                let version = option_env!("CARGO_PKG_VERSION").or(Some("?.?.?")).unwrap();
                Ok(sc_buff!(
                    INFO_BUFF_IDX,
                    split_to_lines(format!(
                        "Mini - A terminal text-editor (v{version})\n\n{INFO_MSG}"
                    )),
                    None,
                ))
            }
            "goto" => {
                let (x, y) = line_column(args);

                let mut pos = self.doc.cur;
                if let Some(x) = x {
                    pos.x = x.saturating_sub(1);
                }
                if let Some(y) = y {
                    pos.y = y.saturating_sub(1);
                }
                cursor::move_to(&mut self.doc, &mut self.doc_view, pos);

                Ok(CommandResult::Ok)
            }
            "s" => Ok(self.search(args)),
            _ => Err(CommandTick::Apply(input)),
        }
    }
}
