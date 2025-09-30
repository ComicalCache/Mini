use crate::util::CommandResult;
use std::io::{BufWriter, Error, Stdout};
use termion::{event::Key, raw::RawTerminal};

pub trait Buffer {
    /// Renders the object to stdout.
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
    fn set_contents(&mut self, contents: &[String]);

    /// Asks if the buffer is ready to quit.
    fn can_quit(&self) -> Result<(), Vec<String>>;
}
