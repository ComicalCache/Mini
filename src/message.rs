use std::fmt::Display;

/// Kind of the message.
#[derive(Clone)]
pub enum MessageKind {
    Info,
    Error,
}

/// A message to be displayed to the user to convey information or show errors.
#[derive(Clone)]
pub struct Message {
    pub kind: MessageKind,

    /// The text of the message.
    pub text: String,
    /// Amount of lines.
    pub lines: usize,

    /// Scrolling offset for long messages.
    pub scroll: usize,
}

impl Message {
    pub fn new(kind: MessageKind, text: String, width: usize) -> Self {
        let mut ret = Self {
            kind,
            text,
            lines: 0,
            scroll: 0,
        };
        ret.calculate_lines(width);

        ret
    }

    /// Calculates the lines of the message with text wrapping.
    pub fn calculate_lines(&mut self, width: usize) {
        let mut lines = 0;
        for line in self.text.lines() {
            // If error messages are too long for usize we have different problems.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            #[allow(clippy::cast_precision_loss)]
            let count = (line.chars().count() as f64 / width as f64).ceil() as usize;
            lines += count.max(1);
        }

        self.lines = lines;
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            MessageKind::Info => writeln!(f, "Info:")?,
            MessageKind::Error => writeln!(f, "Error:")?,
        }
        write!(f, "{}", self.text)
    }
}
