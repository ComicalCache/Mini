pub mod files_buffer;
pub mod info_buffer;
pub mod text_buffer;

#[macro_export]
/// Sends a change buffer result, clearing the motion repeat buffer.
macro_rules! c_buff {
    ($self:ident, $buff:ident) => {{
        use $crate::util::CommandResult;

        $self.base.motion_repeat.clear();
        return CommandResult::ChangeBuffer($buff);
    }};
}
