pub mod files_buffer;
pub mod info_buffer;
pub mod text_buffer;

#[macro_export]
macro_rules! change_buffer {
    ($self:ident, $buff:ident) => {{
        use $crate::util::CommandResult;

        $self.base.motion_repeat.clear();

        return CommandResult::ChangeBuffer($buff);
    }};
}
