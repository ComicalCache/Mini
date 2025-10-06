use crate::{
    INFO_BUFF_IDX, INFO_MSG,
    buffer::base::{BaseBuffer, CommandTick},
    cursor,
    util::{CommandResult, line_column},
};
use std::borrow::Cow;

impl<ModeEnum: Clone, ViewEnum: Clone, CommandEnum: Clone>
    BaseBuffer<ModeEnum, ViewEnum, CommandEnum>
{
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
                cursor::jump_to_line_and_column(&mut self.doc, &mut self.view, x, y);

                Ok(CommandResult::Ok)
            }
            _ => Err(CommandTick::Apply(input)),
        }
    }
}
