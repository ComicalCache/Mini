use crate::buffers::text_buffer::TextBuffer;
use std::borrow::Cow;

const TAB: &str = "    ";

impl TextBuffer {
    /// Inserts a new line above the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_above(&mut self) {
        self.jump_to_beginning_of_line();
        self.doc.insert_line(Cow::from(""));
    }

    /// Inserts a new line bellow the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_bellow(&mut self) {
        self.doc
            .insert_line_at(self.doc.cursor.y + 1, Cow::from(""));
        self.down(1);
    }

    /// Writes a char at the current cursor position.
    /// The cursor will be after the new char.
    pub(super) fn write_char(&mut self, ch: char) {
        self.doc.write_char(ch);
        self.right(1);
    }

    /// Writes a char into the command document.
    /// The cursor will be after the new char.
    pub(super) fn write_cmd_char(&mut self, ch: char) {
        self.cmd.write_char(ch);
        self.cmd_right(1);
    }

    /// Writes a new line character at the current cursor position.
    /// The cursor will be at the beginning of the new line.
    pub(super) fn write_new_line_char(&mut self) {
        let line = &mut self.doc.lines[self.doc.cursor.y];
        let idx = line
            .char_indices()
            .nth(self.doc.cursor.x)
            .map_or(line.len(), |(idx, _)| idx);

        let new_line = line.to_mut().split_off(idx);
        self.doc
            .insert_line_at(self.doc.cursor.y + 1, Cow::from(new_line));

        self.down(1);
        self.left(self.doc.cursor.x);
    }

    /// Writes a tab at the current cursor position.
    /// The cursor will be after the tab.
    pub(super) fn write_tab(&mut self) {
        self.doc.write_str(TAB);
        self.right(TAB.chars().count());
    }

    /// Writes a tab into the command document.
    /// The cursor will be after the tab.
    pub(super) fn write_cmd_tab(&mut self) {
        self.cmd.write_str(TAB);
        self.cmd_right(TAB.chars().count());
    }

    /// Deletes a character at the current cursor position, joining two lines if necessary.
    /// The cursor will be at the delete chars position.
    pub(super) fn delete_char(&mut self) {
        let cursor = self.doc.cursor;

        if cursor.x > 0 {
            // If deleting a character in a line.
            self.left(1);
            self.doc.delete_char();
        } else if cursor.y > 0 {
            // If deleting at the beginning of a line (don't delete the first line).
            let prev_line_len = self.doc.lines[cursor.y - 1].chars().count();
            let line = self.doc.remove_line();
            self.doc.lines[cursor.y - 1].to_mut().push_str(&line);

            self.up(1);
            self.right(prev_line_len);
        }
    }

    /// Deletes a character from the command document.
    /// The cursor will be at the delete chars position.
    pub(super) fn delete_cmd_char(&mut self) {
        let cursor = self.cmd.cursor;

        if cursor.x > 0 {
            // If deleting a character in a line.
            self.cmd_left(1);
            self.cmd.delete_char();
        }
    }

    /// Deletes contents between the selected position and current cursor position
    pub(super) fn delete_selection(&mut self) {
        let Some(pos) = self.selected_pos else {
            return;
        };

        let cursor = self.doc.cursor;
        let (start, end) = if pos <= cursor {
            (pos, cursor)
        } else {
            (cursor, pos)
        };

        if start.y == end.y {
            // Single line deletion

            let line = &mut self.doc.lines[start.y];
            let start_idx = line
                .char_indices()
                .nth(start.x)
                .map_or(line.len(), |(idx, _)| idx);
            let end_idx = line
                .char_indices()
                .nth(end.x)
                .map_or(line.len(), |(idx, _)| idx);

            line.to_mut().drain(start_idx..end_idx);
        } else {
            // Multiple lines were selected

            let (start_line, remaining_lines) = self.doc.lines.split_at_mut(start.y + 1);
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
            start_line[start.y].to_mut().truncate(start_idx);
            start_line[start.y].to_mut().push_str(tail);

            // Remove the inbetween lines
            self.doc.lines.drain((start.y + 1)..=end.y);
        }

        // Allign the cursor
        if cursor.y > start.y {
            let diff = cursor.y - start.y;
            self.up(diff);
        }
        if cursor.x > start.x {
            let diff = cursor.x - start.x;
            self.left(diff);
        } else if cursor.x < start.x {
            let diff = start.x - cursor.x;
            self.right(diff);
        }

        self.selected_pos = None;
        self.doc.edited = true;
    }
}
