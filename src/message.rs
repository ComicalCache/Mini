/// A message to be displayed to the user to convey information or show errors.
pub struct Message {
    /// The text of the message.
    pub text: String,
    /// Amount of lines.
    pub lines: usize,

    /// Scrolling offset for long messages.
    pub scroll: usize,
}

impl Message {
    pub fn new(text: String, width: usize) -> Self {
        let mut ret = Self {
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
