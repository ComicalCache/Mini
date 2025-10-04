use crate::{cursor, custom_buffers::text_buffer::TextBuffer};
use std::borrow::Cow;

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
}
