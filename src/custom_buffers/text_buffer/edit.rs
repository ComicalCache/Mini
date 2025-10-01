use crate::{cursor, custom_buffers::text_buffer::TextBuffer};
use std::borrow::Cow;

const TAB: &str = "    ";

impl TextBuffer {
    /// Inserts a new line above the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_above(&mut self) {
        cursor::jump_to_beginning_of_line(&mut self.doc, &mut self.view);
        self.doc.insert_line(Cow::from(""));
    }

    /// Inserts a new line bellow the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_bellow(&mut self) {
        self.doc.insert_line_at(self.doc.cur.y + 1, Cow::from(""));
        cursor::down(&mut self.doc, &mut self.view, 1);
    }

    /// Writes a char at the current cursor position.
    /// The cursor will be after the new char.
    pub(super) fn write_char(&mut self, ch: char) {
        self.doc.write_char(ch);
        cursor::right(&mut self.doc, &mut self.view, 1);
    }

    /// Writes a char into the command document.
    /// The cursor will be after the new char.
    pub(super) fn write_cmd_char(&mut self, ch: char) {
        self.cmd.write_char(ch);
        cursor::right(&mut self.cmd, &mut self.view, 1);
    }

    /// Writes a new line character at the current cursor position.
    /// The cursor will be at the beginning of the new line.
    pub(super) fn write_new_line_char(&mut self) {
        let line = &mut self.doc.buff[self.doc.cur.y];
        let idx = line
            .char_indices()
            .nth(self.doc.cur.x)
            .map_or(line.len(), |(idx, _)| idx);

        let new_line = line.to_mut().split_off(idx);
        self.doc
            .insert_line_at(self.doc.cur.y + 1, Cow::from(new_line));

        cursor::down(&mut self.doc, &mut self.view, 1);
        let n = self.doc.cur.x;
        cursor::left(&mut self.doc, &mut self.view, n);
    }

    /// Writes a tab at the current cursor position.
    /// The cursor will be after the tab.
    pub(super) fn write_tab(&mut self) {
        self.doc.write_str(TAB);
        cursor::right(&mut self.doc, &mut self.view, TAB.chars().count());
    }

    /// Writes a tab into the command document.
    /// The cursor will be after the tab.
    pub(super) fn write_cmd_tab(&mut self) {
        self.cmd.write_str(TAB);
        cursor::right(&mut self.cmd, &mut self.view, TAB.chars().count());
    }

    /// Deletes a character at the current cursor position, joining two lines if necessary.
    /// The cursor will be at the delete chars position.
    pub(super) fn delete_char(&mut self) {
        let cur = self.doc.cur;

        if cur.x > 0 {
            // If deleting a character in a line.
            cursor::left(&mut self.doc, &mut self.view, 1);
            self.doc.delete_char();
        } else if cur.y > 0 {
            // If deleting at the beginning of a line (don't delete the first line).
            let prev_line_len = self.doc.line_count(cur.y - 1).expect("Illegal state");
            let line = self.doc.remove_line().expect("Illegal state");
            self.doc.buff[cur.y - 1].to_mut().push_str(&line);

            cursor::up(&mut self.doc, &mut self.view, 1);
            cursor::right(&mut self.doc, &mut self.view, prev_line_len);
        }
    }

    /// Deletes a character from the command document.
    /// The cursor will be at the delete chars position.
    pub(super) fn delete_cmd_char(&mut self) {
        if self.cmd.cur.x > 0 {
            // If deleting a character in a line.
            cursor::left(&mut self.cmd, &mut self.view, 1);
            self.cmd.delete_char();
        }
    }
}
