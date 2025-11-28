use crate::{
    buffer::edit::{self, TAB_WIDTH},
    cursor::{self, Cursor},
    custom_buffers::text_buffer::TextBuffer,
    history::{Change, Replace},
    util::Command,
};

impl TextBuffer {
    /// Inserts a new line above the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_above(&mut self) {
        cursor::jump_to_beginning_of_line(&mut self.base.doc, &mut self.base.doc_view);
        self.history.add_change(Change::Insert {
            pos: self.base.doc.cur,
            data: "\n".to_string(),
        });

        self.base.doc.insert_line(self.base.doc.cur.y);
    }

    /// Inserts a new line bellow the current cursor position.
    /// The cursor will be on the new line.
    pub(super) fn insert_move_new_line_bellow(&mut self) {
        let y = self.base.doc.cur.y;
        self.history.add_change(Change::Insert {
            pos: Cursor::new(self.base.doc.line_count(y).unwrap(), y),
            data: "\n".to_string(),
        });

        self.base.doc.insert_line(self.base.doc.cur.y + 1);
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
            .delete_char(self.base.doc.cur.x, self.base.doc.cur.y);

        // Store the change in one replace change.
        {
            let ch = if ch == '\t' {
                " ".repeat(TAB_WIDTH - (self.base.doc.cur.x % TAB_WIDTH))
            } else {
                ch.to_string()
            };

            self.history.add_change(Change::Replace(vec![Replace {
                pos: self.base.doc.cur,
                delete_data: old_ch.to_string(),
                insert_data: ch,
            }]));
        }

        // Pass a None as History to not save the edit again.
        match ch {
            '\t' => edit::write_tab(&mut self.base.doc, &mut self.base.doc_view, None, true),
            _ => self
                .base
                .doc
                .write_char(ch, self.base.doc.cur.x, self.base.doc.cur.y),
        }
    }

    /// Paste the system clipboard contents after the current cursor.
    pub(super) fn paste(&mut self, trim_newline: bool, move_to: bool) -> Option<Command> {
        let mut data = match self.base.clipboard.get_text() {
            Ok(content) => content,
            Err(err) => {
                return Some(Command::Error(err.to_string()));
            }
        };

        if trim_newline && data.ends_with('\n') {
            data.truncate(data.len() - 1);
        }

        self.base.doc.write_str(data.as_str());
        let pos = self.base.doc.cur;
        if move_to {
            let end_pos = cursor::end_pos(&pos, data.as_str());
            cursor::move_to(&mut self.base.doc, &mut self.base.doc_view, end_pos);
        }

        self.history.add_change(Change::Insert { pos, data });

        None
    }
}
