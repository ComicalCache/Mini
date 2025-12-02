use crate::shell_command::performer::Performer;
use std::io::{Error, IntoInnerError, LineWriter, Write};
use vte::Parser;

pub(super) struct Writer<W: Write> {
    performer: Performer<W>,
    parser: Parser,
}

impl<W: Write> Writer<W> {
    pub fn new(inner: W) -> Self {
        Self {
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
        self.performer.err.take().map_or(Ok(buf.len()), Err)
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.performer.flush()
    }
}
