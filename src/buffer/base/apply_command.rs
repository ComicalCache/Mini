use crate::{
    INFO_BUFF_IDX, INFO_MSG,
    buffer::base::{BaseBuffer, CommandTick},
    cursor,
    util::{CommandResult, line_column},
};
use regex::Regex;
use std::borrow::Cow;

impl<ModeEnum: Clone, ViewEnum: Clone, CommandEnum: Clone>
    BaseBuffer<ModeEnum, ViewEnum, CommandEnum>
{
    fn search(&mut self, args: &str) -> CommandResult {
        let Some(end) = self.sel else {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from("Search command requires a selection.")],
                None,
            );
        };

        if !args.starts_with('/') || !args.ends_with('/') || args.len() == 2 {
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from(
                    "Expected a valid regular expression like '/<regex>/'",
                )],
                None,
            );
        }

        let regex = match Regex::new(&args[1..args.len() - 1]) {
            Ok(regex) => regex,
            Err(err) => {
                let mut buff = vec![Cow::from(format!(
                    "'{args}' is not a valid regular expression:",
                ))];
                buff.extend(err.to_string().lines().map(str::to_string).map(Cow::from));
                return CommandResult::SetAndChangeBuffer(INFO_BUFF_IDX, buff, None);
            }
        };

        let start = self.doc.cur;
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
            return CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                vec![Cow::from("No matches found")],
                None,
            );
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

        //let cmd1 = cmd.clone();
        let (cmd, args) = match input.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (input.trim(), ""),
        };

        match cmd {
            "q" => Ok(CommandResult::Quit),
            "qq" => Ok(CommandResult::ForceQuit),
            "?" => Ok(CommandResult::SetAndChangeBuffer(
                INFO_BUFF_IDX,
                INFO_MSG.lines().map(Cow::from).collect(),
                None,
            )),
            "goto" => {
                let (x, y) = line_column(args);
                cursor::jump_to_line_and_column(&mut self.doc, &mut self.doc_view, x, y);

                Ok(CommandResult::Ok)
            }
            "s" => Ok(self.search(args)),
            _ => Err(CommandTick::Apply(input)),
        }
    }
}
