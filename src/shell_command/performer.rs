use std::io::{Error, IntoInnerError, LineWriter, Write};
use vte::Perform;

pub(super) struct Performer<W: Write> {
    pub(super) writer: LineWriter<W>,
    pub(super) err: Option<Error>,
}

impl<W: Write> Performer<W> {
    pub fn flush(&mut self) -> Result<(), Error> {
        self.writer.flush()
    }

    pub fn into_inner(self) -> Result<W, IntoInnerError<LineWriter<W>>> {
        self.writer.into_inner()
    }
}

impl<W: Write> Perform for Performer<W> {
    fn print(&mut self, c: char) {
        self.err = write!(self.writer, "{c}").err();
    }

    fn execute(&mut self, byte: u8) {
        if byte == b'\n' || byte == b'\r' {
            self.err = writeln!(self.writer).err();
        }
    }
}
