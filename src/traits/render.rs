use std::io::{BufWriter, Error, Stdout};
use termion::raw::RawTerminal;

pub trait Render {
    fn render(&mut self, stdout: &mut BufWriter<RawTerminal<Stdout>>) -> Result<(), Error>;
    fn resize(&mut self, w: usize, h: usize);
}
