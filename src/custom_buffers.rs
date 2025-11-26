pub mod files_buffer;
pub mod text_buffer;

/// Sends a change buffer result, clearing the motion repeat buffer.
#[macro_export]
macro_rules! c_buff {
    ($self:ident, $buff:ident) => {{
        use $crate::util::CommandResult;
        return CommandResult::ChangeBuffer($buff);
    }};
}

/// `SetAndChangeBuffer`
#[macro_export]
macro_rules! sc_buff {
    ($self:ident, $buff:ident, $contents:expr $(,)?) => {{
        use $crate::util::CommandResult;

        CommandResult::SetAndChangeBuffer($buff, $contents, None, None)
    }};
    ($self:ident, $buff:ident, $contents:expr, $path:expr, $file_name:expr $(,)?) => {{
        use $crate::util::CommandResult;

        CommandResult::SetAndChangeBuffer($buff, $contents, $path, $file_name)
    }};
}
