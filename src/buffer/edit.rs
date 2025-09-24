use crate::{buffer::Buffer, util::CursorMove};

const TAB: &str = "    ";

impl Buffer {
    /// Inserts a new line above the current line and moves to it
    pub fn insert_move_new_line_above(&mut self) {
        self.line_buff.insert(self.txt_pos.y, String::new());
        // No need to move since the cursor pos stays the same
        self.edited = true;
    }

    /// Inserts a new line bellow the current line and moves to it
    pub fn insert_move_new_line_bellow(&mut self) {
        self.line_buff.insert(self.txt_pos.y + 1, String::new());
        self.move_cursor(CursorMove::Down, 1);
        self.edited = true;
    }

    /// Writes a character to the buffer
    pub fn write_char(&mut self, ch: char) {
        let idx = self.line_buff[self.txt_pos.y]
            .char_indices()
            .nth(self.txt_pos.x)
            .map_or(self.line_buff[self.txt_pos.y].len(), |(idx, _)| idx);
        self.line_buff[self.txt_pos.y].insert(idx, ch);

        self.txt_pos.x += 1;
        self.term_content_pos.x = (self.term_content_pos.x + 1).min(self.screen_dims.w);
        self.edited = true;
    }

    /// Writes a character to the command buffer
    pub fn write_cmd_char(&mut self, ch: char) {
        let idx = self
            .cmd_buff
            .char_indices()
            .nth(self.cmd_pos.x)
            .map_or(self.cmd_buff.len(), |(idx, _)| idx);
        self.cmd_buff.insert(idx, ch);

        self.cmd_pos.x += 1;
        self.term_cmd_pos.x = (self.cmd_pos.x + 1).min(self.screen_dims.w);
    }

    /// Writes a new line to the buffer, splitting an existing line if necessary
    pub fn write_new_line(&mut self) {
        let line = &mut self.line_buff[self.txt_pos.y];
        let idx = line
            .char_indices()
            .nth(self.txt_pos.x)
            .map_or(line.len(), |(idx, _)| idx);

        let new_line = line.split_off(idx);
        self.line_buff.insert(self.txt_pos.y + 1, new_line);

        self.move_cursor(CursorMove::Down, 1);
        self.move_cursor(CursorMove::Left, self.txt_pos.x);
        self.edited = true;
    }

    /// Writes a tab character to the buffer
    pub fn write_tab(&mut self) {
        let idx = self.line_buff[self.txt_pos.y]
            .char_indices()
            .nth(self.txt_pos.x)
            .map_or(self.line_buff[self.txt_pos.y].len(), |(idx, _)| idx);
        self.line_buff[self.txt_pos.y].insert_str(idx, TAB);

        self.move_cursor(CursorMove::Right, TAB.chars().count());
        self.edited = true;
    }

    /// Writes a tab character to the command buffer
    pub fn write_cmd_tab(&mut self) {
        let idx = self
            .cmd_buff
            .char_indices()
            .nth(self.cmd_pos.x)
            .map_or(self.cmd_buff.len(), |(idx, _)| idx);
        self.cmd_buff.insert_str(idx, TAB);

        self.move_cmd_cursor(CursorMove::Right, TAB.chars().count());
    }

    /// Deletes a character from the buffer, joining two lines if necessary
    pub fn delete_char(&mut self) {
        if self.txt_pos.x > 0 {
            // If deleting a character in a line
            let line = &mut self.line_buff[self.txt_pos.y];
            let idx = line
                .char_indices()
                .nth(self.txt_pos.x - 1)
                .map(|(idx, _)| idx)
                // Safe to unwrap
                .unwrap();

            line.remove(idx);
            self.move_cursor(CursorMove::Left, 1);
            self.edited = true;
        } else if self.txt_pos.y > 0 {
            // If deleting at the beginning of a line (don't delete the first line)
            let prev_line_len = self.line_buff[self.txt_pos.y - 1].chars().count();
            let line = self.line_buff.remove(self.txt_pos.y);
            self.line_buff[self.txt_pos.y - 1].push_str(&line);

            self.move_cursor(CursorMove::Up, 1);
            self.move_cursor(CursorMove::Right, prev_line_len);
            self.edited = true;
        }
    }

    /// Deletes a character from the command buffer
    pub fn delete_cmd_char(&mut self) {
        if self.cmd_pos.x > 0 {
            let idx = self
                .cmd_buff
                .char_indices()
                .nth(self.cmd_pos.x - 1)
                .map(|(idx, _)| idx)
                // Safe to unwrap
                .unwrap();

            self.cmd_buff.remove(idx);
            self.move_cmd_cursor(CursorMove::Left, 1);
        }
    }

    /// Deletes the text between the selection point and current cursor
    pub fn delete_selection(&mut self, inclusive: bool) {
        let Some(pos) = self.select else {
            return;
        };

        let (start, mut end) = if pos <= self.txt_pos {
            (pos, self.txt_pos)
        } else {
            (self.txt_pos, pos)
        };

        if inclusive {
            let line = &self.line_buff[end.y];
            if end.x < line.chars().count() {
                end.x += 1;
            }
        }

        if start.y == end.y {
            // Single line deletion

            let line = &mut self.line_buff[start.y];
            let start_idx = line
                .char_indices()
                .nth(start.x)
                .map_or(line.len(), |(idx, _)| idx);
            let end_idx = line
                .char_indices()
                .nth(end.x)
                .map_or(line.len(), |(idx, _)| idx);

            line.drain(start_idx..end_idx);
        } else {
            // Multiple lines were selected

            let (start_line, remaining_lines) = self.line_buff.split_at_mut(start.y + 1);
            let end_line = &remaining_lines[end.y - (start.y + 1)];
            let tail = {
                let start_idx = end_line
                    .char_indices()
                    .nth(end.x)
                    .map_or(end_line.len(), |(idx, _)| idx);
                &end_line[start_idx..]
            };

            let start_idx = start_line[start.y]
                .char_indices()
                .nth(start.x)
                .map_or(start_line[start.y].len(), |(idx, _)| idx);
            start_line[start.y].truncate(start_idx);
            start_line[start.y].push_str(tail);

            // Remove the inbetween lines
            self.line_buff.drain((start.y + 1)..=end.y);
        }
        self.select = None;

        // Allign the cursor
        if self.txt_pos.y > start.y {
            let diff = self.txt_pos.y - start.y;
            self.move_cursor(CursorMove::Up, diff);
        }
        if self.txt_pos.x > start.x {
            let diff = self.txt_pos.x - start.x;
            self.move_cursor(CursorMove::Left, diff);
        } else if self.txt_pos.x < start.x {
            let diff = start.x - self.txt_pos.x;
            self.move_cursor(CursorMove::Right, diff);
        }
    }
}
