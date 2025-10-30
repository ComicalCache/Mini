use crate::{
    INFO_BUFF_IDX,
    buffer::{
        edit,
        history::{Change, Replace},
    },
    cursor::{self, Cursor},
    custom_buffers::text_buffer::TextBuffer,
    sc_buff,
    util::{CommandResult, split_to_lines},
};
use std::borrow::Cow;

impl TextBuffer {
    /// Inserts a new line above the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_above(&mut self) {
        cursor::jump_to_beginning_of_line(&mut self.base.doc, &mut self.base.doc_view);
        self.history.add_change(Change::Insert {
            pos: self.base.doc.cur,
            data: Cow::from("\n"),
        });

        self.base
            .doc
            .insert_line(Cow::from(""), self.base.doc.cur.y);
    }

    /// Inserts a new line bellow the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_bellow(&mut self) {
        let y = self.base.doc.cur.y;
        self.history.add_change(Change::Insert {
            pos: Cursor::new(self.base.doc.line_count(y).unwrap(), y),
            data: Cow::from("\n"),
        });

        self.base
            .doc
            .insert_line(Cow::from(""), self.base.doc.cur.y + 1);
        cursor::down(&mut self.base.doc, &mut self.base.doc_view, 1);

        // Set target x coordinate, otherwise it would snap back when moving without inserting.
        cursor::left(&mut self.base.doc, &mut self.base.doc_view, 0);
    }

    /// Replaces a character at the current cursor position.
    pub(super) fn replace(&mut self, ch: char) {
        if self.base.doc.line_count(self.base.doc.cur.y).unwrap() <= self.base.doc.cur.x {
            return;
        }

        let old_ch = self
            .base
            .doc
            .delete_char(self.base.doc.cur.x, self.base.doc.cur.y)
            .unwrap();

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
                let _ = self.history.undo();
            }
            '\t' => {
                edit::write_tab(
                    &mut self.base.doc,
                    &mut self.base.doc_view,
                    Some(&mut self.history),
                );

                // Pop the change added by the write method above.
                let _ = self.history.undo();
            }
            _ => self
                .base
                .doc
                .write_char(ch, self.base.doc.cur.x, self.base.doc.cur.y),
        }
    }

    /// Paste the system clipboard contents after the current cursor.
    pub(super) fn paste(&mut self, trim_newline: bool) -> Option<CommandResult> {
        let mut content = match self.base.clipboard.get_text() {
            Ok(content) => content,
            Err(err) => {
                return Some(sc_buff!(
                    INFO_BUFF_IDX,
                    split_to_lines(err.to_string()),
                    None,
                ));
            }
        };

        if trim_newline && content.ends_with('\n') {
            content.truncate(content.len() - 1);
        }

        self.base.doc.write_str(&content);

        self.history.add_change(Change::Insert {
            pos: self.base.doc.cur,
            data: Cow::from(content),
        });

        None
    }
}
