use crate::{
    INFO_BUFF_IDX,
    buffer::{
        edit,
        history::{Change, Replace},
    },
    cursor,
    custom_buffers::text_buffer::TextBuffer,
    util::{CommandResult, split_to_lines},
};
use std::borrow::Cow;

impl TextBuffer {
    /// Inserts a new line above the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_above(&mut self) {
        cursor::jump_to_beginning_of_line(&mut self.base.doc, &mut self.base.doc_view);
        self.base.doc.insert_line(Cow::from(""));
    }

    /// Inserts a new line bellow the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_bellow(&mut self) {
        self.base
            .doc
            .insert_line_at(self.base.doc.cur.y + 1, Cow::from(""));
        cursor::down(&mut self.base.doc, &mut self.base.doc_view, 1);

        // Set target x coordinate.
        cursor::left(&mut self.base.doc, &mut self.base.doc_view, 0);
    }

    /// Replaces a character at the current cursor position.
    pub(super) fn replace(&mut self, ch: char) {
        let Some(old_ch) = self.base.doc.delete_char() else {
            return;
        };

        self.history.add_change(Change::Replace(vec![Replace {
            pos: self.base.doc.cur,
            delete_data: Cow::from(old_ch.to_string()),
            insert_data: Cow::from(ch.to_string()),
        }]));

        match ch {
            '\n' => {
                edit::write_new_line_char(
                    &mut self.base.doc,
                    &mut self.base.doc_view,
                    Some(&mut self.history),
                );

                // Pop the change added by the write method above.
                self.history.pop_change();
            }
            '\t' => {
                edit::write_tab(
                    &mut self.base.doc,
                    &mut self.base.doc_view,
                    Some(&mut self.history),
                );

                // Pop the change added by the write method above.
                self.history.pop_change();
            }
            _ => self.base.doc.write_char(ch),
        }
    }

    /// Paste the system clipboard contents after the current cursor.
    pub(super) fn paste(&mut self) -> Option<CommandResult> {
        let content = match self.base.clipboard.get_text() {
            Ok(content) => content,
            Err(err) => {
                self.base.motion_repeat.clear();
                return Some(CommandResult::SetAndChangeBuffer(
                    INFO_BUFF_IDX,
                    split_to_lines(err.to_string()),
                    None,
                ));
            }
        };

        self.base.doc.write_str(&content);

        self.history.add_change(Change::Insert {
            pos: self.base.doc.cur,
            data: Cow::from(content),
        });

        None
    }
}
