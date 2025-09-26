use crate::text_buffer::TextBuffer;

const TAB: &str = "    ";

impl TextBuffer {
    pub(super) fn insert_move_new_line_above(&mut self) {
        self.doc.insert_line(String::new());
        // No need to move since the cursor pos stays the same
    }

    pub(super) fn insert_move_new_line_bellow(&mut self) {
        self.doc
            .insert_line_at(self.doc.cursor.y + 1, String::new());
        self.down(1);
    }

    pub(super) fn write_char(&mut self, ch: char) {
        self.doc.write_char(ch);
        self.right(1);
    }

    pub(super) fn write_new_line_char(&mut self) {
        let line = &mut self.doc.lines[self.doc.cursor.y];
        let idx = line
            .char_indices()
            .nth(self.doc.cursor.x)
            .map_or(line.len(), |(idx, _)| idx);

        let new_line = line.split_off(idx);
        self.doc.insert_line_at(self.doc.cursor.y + 1, new_line);

        self.down(1);
        self.left(self.doc.cursor.x);
    }

    pub(super) fn write_tab(&mut self) {
        self.doc.write_str(TAB);
        self.right(TAB.chars().count());
    }

    /// Deletes a character from the buffer, joining two lines if necessary
    pub(super) fn delete_char(&mut self) {
        let cursor = self.doc.cursor;

        if cursor.x > 0 {
            // If deleting a character in a line
            self.doc.delete_char_at(cursor.x - 1, cursor.y);
            self.left(1);
        } else if cursor.y > 0 {
            // If deleting at the beginning of a line (don't delete the first line)
            let prev_line_len = self.doc.lines[cursor.y - 1].chars().count();
            let line = self.doc.remove_line();
            self.doc.lines[cursor.y - 1].push_str(&line);

            self.up(1);
            self.right(prev_line_len);
        }
    }

    /*pub(super) fn delete_selection(&mut self, inclusive: bool) {
        let Some(pos) = self.select else {
            return;
        };

        let (start, mut end) = if pos <= self.txt_pos {
            (pos, self.txt_pos)
        } else {
            (self.txt_pos, pos)
        };

        if inclusive {
            end.x = (end.x + 1).min(self.line_buff[end.y].chars().count());
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
    }*/
}
