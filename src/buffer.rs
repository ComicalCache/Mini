pub mod base;
pub mod delete;
pub mod edit;
pub mod yank;

use crate::util::CommandResult;
use std::{
    borrow::Cow,
    io::{BufWriter, Error, Stdout},
    path::PathBuf,
};
use termion::{event::Key, raw::RawTerminal};

/// The buffer trait defines the basic primitives a buffer needs.
pub trait Buffer {
    /// Checks if the buffer needs to be rerendered.
    fn need_rerender(&self) -> bool;

    /// Renders the buffer to stdout.
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error>;

    /// Handles the event, that the terminal was resized.
    fn resize(&mut self, w: usize, h: usize);

    /// Processes one tick. A tick is either:
    /// - an immediate tick on input with the corresponding key
    /// - a periodic empty tick on no input
    ///
    /// Thus it should not be assuemed that a tick is always of periodic nature.
    fn tick(&mut self, key: Option<Key>) -> CommandResult;

    /// Sets the contents of a buffer.
    fn set_contents(&mut self, contents: &[Cow<'static, str>], path: Option<PathBuf>);

    /// Asks if the buffer is ready to quit/has pending changes.
    fn can_quit(&self) -> Result<(), Vec<Cow<'static, str>>>;
}
