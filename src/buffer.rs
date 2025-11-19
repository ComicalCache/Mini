pub mod base;
pub mod delete;
pub mod edit;
pub mod yank;

use crate::{display::Display, util::CommandResult};
use std::path::PathBuf;
use termion::event::Key;

/// The buffer trait defines the basic primitives a buffer needs.
pub trait Buffer {
    /// Checks if the buffer needs to be rerendered.
    fn need_rerender(&self) -> bool;

    /// Applies syntax highlighting on the buffer contents.
    fn highlight(&mut self);

    /// Renders the buffer to a `Display`.
    fn render(&mut self, display: &mut Display);

    /// Handles the event, that the terminal was resized.
    fn resize(&mut self, w: usize, h: usize, x_off: usize, y_off: usize);

    /// Processes one tick. A tick is either:
    /// - an immediate tick on input with the corresponding key
    /// - a periodic empty tick on no input
    ///
    /// Thus it should not be assuemed that a tick is always of periodic nature.
    fn tick(&mut self, key: Option<Key>) -> CommandResult;

    /// Sets the contents of a buffer.
    fn set_contents(&mut self, contents: String, path: Option<PathBuf>, file_name: Option<String>);

    /// Asks if the buffer is ready to quit/has pending changes.
    fn can_quit(&self) -> Result<(), String>;
}
