use crate::{buffer::history::Change, cursor, custom_buffers::text_buffer::TextBuffer};

impl TextBuffer {
    /// Undos the last change if one exists.
    pub(super) fn undo(&mut self) {
        let Some(change) = self.history.undo() else {
            return;
        };

        match &change {
            Change::Insert { pos, data } => {
                // To undo an insert, delete the data that was inserted.
                let end_pos = cursor::end_pos(pos, data);
                self.base.doc.remove_range(*pos, end_pos);
                cursor::move_to(&mut self.base.doc, &mut self.base.view, *pos);
            }
            Change::Delete { pos, data } => {
                // To undo a delete, insert the data back.
                self.base.doc.write_str_at(pos.x, pos.y, data);
                cursor::move_to(
                    &mut self.base.doc,
                    &mut self.base.view,
                    cursor::end_pos(pos, data),
                );
            }
        }

        self.history.push_redo(change);
    }

    /// Redos the last undo, if one exists.
    pub(super) fn redo(&mut self) {
        let Some(change) = self.history.redo() else {
            return;
        };

        match &change {
            Change::Insert { pos, data } => {
                // To redo an insert, insert the data.
                self.base.doc.write_str_at(pos.x, pos.y, data);
                cursor::move_to(
                    &mut self.base.doc,
                    &mut self.base.view,
                    cursor::end_pos(pos, data),
                );
            }
            Change::Delete { pos, data } => {
                // To redo a delete, delete the data.
                self.base.doc.remove_range(*pos, cursor::end_pos(pos, data));
                cursor::move_to(&mut self.base.doc, &mut self.base.view, *pos);
            }
        }

        self.history.push_undo(change);
    }
}
