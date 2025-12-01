use crate::shell_command::performer::Performer;
use std::io::{Error, IntoInnerError, LineWriter, Write};
use vte::Parser;

pub(super) struct Writer<W: Write> {
    performer: Performer<W>,
    parser: Parser,
}

impl<W: Write> Writer<W> {
    pub fn new(inner: W) -> Writer<W> {
        Writer {
            performer: Performer {
                writer: LineWriter::new(inner),
                err: None,
            },
            parser: Parser::new(),
        }
    }

    pub fn into_inner(self) -> Result<W, IntoInnerError<LineWriter<W>>> {
        self.performer.into_inner()
    }
}

impl<W: Write> Write for Writer<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.parser.advance(&mut self.performer, buf);
        match self.performer.err.take() {
            Some(e) => Err(e),
            None => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.performer.flush()
    }
}
