use crate::{buffer::Buffer, util::Mode};
use std::io::{BufWriter, Error, Stdout, Write};
use termion::{
    clear::All,
    cursor::{BlinkingBar, BlinkingBlock, Goto},
    raw::RawTerminal,
};

impl Buffer {
    fn set_text_line(&mut self, screen_idx: usize, lines_idx: usize) {
        let line = &self.line_buff[lines_idx];
        // Minus one since terminal coordinates start at 1
        let lower = self.txt_pos.x.saturating_sub(self.term_content_pos.x - 1);
        // Only print lines if their content is visible on the screen (horizontal movement)
        if line.chars().count() > lower {
            let upper = (lower + self.screen_dims.w).min(line.chars().count());
            let start = line.char_indices().nth(lower).map(|(idx, _)| idx).unwrap();
            let end = line
                .char_indices()
                .nth(upper)
                // Use all remaining bytes if they don't fill the entire line
                .map_or(line.len(), |(idx, _)| idx);

            self.screen_buff[screen_idx].replace_range(.., &line[start..end]);
        }
    }

    fn set_info_line(&mut self, screen_idx: usize) -> Result<(), std::fmt::Error> {
        use std::fmt::Write;

        let mode = match self.mode {
            Mode::View => "V",
            Mode::Write => "W",
            Mode::Command => "C",
        };
        // Plus 1 since text coordinates are 0 indexed
        let line = self.txt_pos.y + 1;
        let col = self.txt_pos.x + 1;
        let total = self.line_buff.len();
        let percentage = 100 * (self.txt_pos.y + 1) / self.line_buff.len();
        let size: usize = self.line_buff.iter().map(String::len).sum();
        let buff_name = self.buff_name;

        self.screen_buff[screen_idx].clear();
        write!(
            &mut self.screen_buff[screen_idx],
            "[{buff_name}] [{mode}] [{line}:{col}/{total} {percentage}%] [{size}B]",
        )?;
        if let Some(pos) = self.select {
            // Plus 1 since text coordinates are 0 indexed
            let line = pos.y + 1;
            let col = pos.x + 1;
            write!(
                &mut self.screen_buff[screen_idx],
                " [Selected {line}:{col}]"
            )?;
        }

        let edited = if self.edited { '*' } else { ' ' };
        write!(&mut self.screen_buff[screen_idx], " {edited}")
    }

    fn set_cmd_line(&mut self, screen_idx: usize) {
        self.screen_buff[screen_idx].clone_from(&self.cmd_buff);
    }

    // Prints the current buffer to the screen
    pub fn print_screen(
        &mut self,
        stdout: &mut BufWriter<RawTerminal<Stdout>>,
    ) -> Result<(), Error> {
        // Set info line
        if let Err(err) = self.set_info_line(0) {
            return Err(Error::other(err));
        }

        // Calculate which line of text is visible at what line on the screen
        #[allow(clippy::cast_possible_wrap)]
        let lines_offset = (self.txt_pos.y + 1) as isize - self.term_content_pos.y as isize;

        // Plus one for info line offset
        for (screen_idx, lines_idx) in (1..self.screen_dims.h).zip(lines_offset + 1..) {
            self.screen_buff[screen_idx].clear();

            // Skip screen lines outside the text line bounds
            // The value is guaranteed positive at that point
            #[allow(clippy::cast_sign_loss)]
            if lines_idx < 0 || (lines_idx as usize) >= self.line_buff.len() {
                continue;
            }

            // The value is guaranteed positive at that point
            #[allow(clippy::cast_sign_loss)]
            self.set_text_line(screen_idx, lines_idx as usize);
        }

        // Set command line if in command mode
        if self.mode == Mode::Command {
            self.set_cmd_line(self.screen_dims.h - 1);
        }

        // Write the new content
        write!(stdout, "{All}{}", Goto(1, 1))?;
        for line in &self.screen_buff[..self.screen_buff.len() - 1] {
            write!(stdout, "{line}\n\r")?;
        }
        write!(stdout, "{}", self.screen_buff[self.screen_buff.len() - 1])?;

        // Set the cursor to represent the current input mode
        // Since term_pos is always bounded by the screen dimensions it will never truncate
        #[allow(clippy::cast_possible_truncation)]
        match self.mode {
            Mode::View => write!(
                stdout,
                "{}{BlinkingBlock}",
                Goto(
                    self.term_content_pos.x as u16,
                    self.term_content_pos.y as u16
                )
            )?,
            Mode::Write => write!(
                stdout,
                "{}{BlinkingBar}",
                Goto(
                    self.term_content_pos.x as u16,
                    self.term_content_pos.y as u16
                )
            )?,
            Mode::Command => write!(
                stdout,
                "{}{BlinkingBar}",
                Goto(self.term_cmd_pos.x as u16, self.term_cmd_pos.y as u16)
            )?,
        }

        stdout.flush()
    }
}
