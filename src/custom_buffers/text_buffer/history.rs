use crate::{cursor, custom_buffers::text_buffer::TextBuffer, history::Change};

impl TextBuffer {
    /// Undos the last change if one exists.
    pub(super) fn undo(&mut self) {
        let Some(change) = self.history.undo() else {
            return;
        };

        match &change {
            Change::Insert { pos, data } => {
                // To undo an insert, delete the data that was inserted.
                self.base.doc.remove_range(*pos, cursor::end_pos(pos, data));
                cursor::move_to(&mut self.base.doc, &mut self.base.doc_view, *pos);
            }
            Change::Delete { pos, data } => {
                // To undo a delete, insert the data back.
                self.base.doc.write_str_at(pos.x, pos.y, data);
                cursor::move_to(
                    &mut self.base.doc,
                    &mut self.base.doc_view,
                    cursor::end_pos(pos, data),
                );
            }
            Change::Replace(events) => {
                // Undo in reverse order to not change indices of later events.
                for e in events.iter().rev() {
                    // To undo an insert, delete the data that was inserted.
                    self.base
                        .doc
                        .remove_range(e.pos, cursor::end_pos(&e.pos, &e.insert_data));
                    cursor::move_to(&mut self.base.doc, &mut self.base.doc_view, e.pos);

                    // To undo a delete, insert the data back.
                    self.base.doc.write_str_at(e.pos.x, e.pos.y, &e.delete_data);
                    cursor::move_to(
                        &mut self.base.doc,
                        &mut self.base.doc_view,
                        cursor::end_pos(&e.pos, &e.delete_data),
                    );
                }
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
                    &mut self.base.doc_view,
                    cursor::end_pos(pos, data),
                );
            }
            Change::Delete { pos, data } => {
                // To redo a delete, delete the data.
                self.base.doc.remove_range(*pos, cursor::end_pos(pos, data));
                cursor::move_to(&mut self.base.doc, &mut self.base.doc_view, *pos);
            }
            Change::Replace(events) => {
                for e in events {
                    // To redo a delete, delete the data.
                    self.base
                        .doc
                        .remove_range(e.pos, cursor::end_pos(&e.pos, &e.delete_data));
                    cursor::move_to(&mut self.base.doc, &mut self.base.doc_view, e.pos);

                    // To redo an insert, insert the data.
                    self.base.doc.write_str_at(e.pos.x, e.pos.y, &e.insert_data);
                    cursor::move_to(
                        &mut self.base.doc,
                        &mut self.base.doc_view,
                        cursor::end_pos(&e.pos, &e.insert_data),
                    );
                }
            }
        }

        self.history.push_undo(change);
    }
}
