use std::io::{BufWriter, Error, Stdout};
use termion::raw::RawTerminal;

pub trait Render {
    /// Renders the object to stdout.
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error>;

    /// Handles the event, that the terminal was resized.
    fn resize(&mut self, w: usize, h: usize);
}
