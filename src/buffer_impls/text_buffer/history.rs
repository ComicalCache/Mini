use crate::{buffer_impls::text_buffer::TextBuffer, cursor};

impl TextBuffer {
    /// Undos the last change if one exists.
    pub(super) fn undo(&mut self) {
        let Some(changes) = self.history.undo() else {
            return;
        };

        // Undo in reverse order to not change indices of later events.
        for c in changes.iter().rev() {
            // To undo an insert, delete the data that was inserted.
            self.base
                .doc
                .remove_range(c.pos, cursor::pos_after_text(&c.pos, &c.insert_data));
            cursor::move_to(&mut self.base.doc, c.pos);

            // To undo a delete, insert the data back.
            self.base.doc.write_str_at(c.pos.x, c.pos.y, &c.delete_data);
            cursor::move_to(
                &mut self.base.doc,
                cursor::pos_after_text(&c.pos, &c.delete_data),
            );
        }

        self.history.push_redo(changes);
    }

    /// Redos the last undo, if one exists.
    pub(super) fn redo(&mut self) {
        let Some(changes) = self.history.redo() else {
            return;
        };

        for c in &changes {
            // To redo a delete, delete the data.
            self.base
                .doc
                .remove_range(c.pos, cursor::pos_after_text(&c.pos, &c.delete_data));
            cursor::move_to(&mut self.base.doc, c.pos);

            // To redo an insert, insert the data.
            self.base.doc.write_str_at(c.pos.x, c.pos.y, &c.insert_data);
            cursor::move_to(
                &mut self.base.doc,
                cursor::pos_after_text(&c.pos, &c.insert_data),
            );
        }

        self.history.push_undo(changes);
    }
}
