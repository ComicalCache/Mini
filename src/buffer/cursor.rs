use crate::{buffer::Buffer, util::CursorMove};

impl Buffer {
    /// Handles a cursor move.
    pub fn move_cursor(&mut self, cursor_move: CursorMove, n: usize) {
        if n == 0 {
            return;
        }

        match cursor_move {
            CursorMove::Left => {
                self.term_content_pos.x = self.term_content_pos.x.saturating_sub(n).max(1);
                self.txt_pos.x = self.txt_pos.x.saturating_sub(n);
            }
            CursorMove::Down => {
                // Only move down if there is more text available
                let text_bound = self.line_buff.len();
                if self.txt_pos.y + n < text_bound {
                    self.txt_pos.y = (self.txt_pos.y + n).min(text_bound.saturating_sub(1));
                }
            }
            CursorMove::Up => {
                self.txt_pos.y = self.txt_pos.y.saturating_sub(n);
            }
            CursorMove::Right => {
                // Only move right if there is more text available
                let line_bound = self.line_buff[self.txt_pos.y].chars().count();
                if self.txt_pos.x + n <= line_bound {
                    self.term_content_pos.x = (self.term_content_pos.x + n).min(self.screen_dims.w);
                    self.txt_pos.x = (self.txt_pos.x + n).min(line_bound);
                }
            }
        }

        // When moving up and down, handle the case that one line contains less text than the current
        let line_bound = self.line_buff[self.txt_pos.y].chars().count();
        if (cursor_move == CursorMove::Down || cursor_move == CursorMove::Up)
            && self.txt_pos.x >= line_bound
        {
            let diff = self.txt_pos.x - line_bound;
            self.txt_pos.x = line_bound;
            self.term_content_pos.x = (self.term_content_pos.x.saturating_sub(diff)).max(1);
        }
    }

    /// Moves the cursor when in command mode
    pub fn move_cmd_cursor(&mut self, cursor_move: CursorMove, n: usize) {
        self.term_cmd_pos.y = self.screen_dims.h;

        match cursor_move {
            CursorMove::Left => {
                self.term_cmd_pos.x = self.term_cmd_pos.x.saturating_sub(n).max(1);
                self.cmd_pos.x = self.cmd_pos.x.saturating_sub(n);
            }
            CursorMove::Right => {
                self.term_cmd_pos.x = (self.term_cmd_pos.x + n).min(self.screen_dims.w);
                self.cmd_pos.x = (self.cmd_pos.x + n).min(self.cmd_buff.chars().count());
            }
            _ => {
                // TODO: add command history to scroll
            }
        }
    }
}
