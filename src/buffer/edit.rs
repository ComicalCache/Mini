use crate::{
    buffer::history::{Change, History},
    cursor,
    document::Document,
    viewport::Viewport,
};
use std::borrow::Cow;

pub const TAB_WIDTH: usize = 4;

/// Writes a char at the current cursor position.
/// The cursor will be after the new char.
pub fn write_char(
    doc: &mut Document,
    view: &mut Viewport,
    history: Option<&mut History>,
    ch: char,
) {
    if let Some(history) = history {
        history.add_change(Change::Insert {
            pos: doc.cur,
            data: Cow::from(ch.to_string()),
        });
    }

    doc.write_char(ch, doc.cur.x, doc.cur.y);

    if ch != '\n' {
        cursor::right(doc, view, 1);
    } else {
        cursor::down(doc, view, 1);
        cursor::jump_to_beginning_of_line(doc, view);
    }
}

/// Writes a tab at the current cursor position.
/// The cursor will be after the tab.
pub fn write_tab(
    doc: &mut Document,
    view: &mut Viewport,
    history: Option<&mut History>,
    relative: bool,
) {
    let n = if relative {
        TAB_WIDTH - (doc.cur.x % TAB_WIDTH)
    } else {
        TAB_WIDTH
    };
    let spaces = " ".repeat(n);

    if let Some(history) = history {
        history.add_change(Change::Insert {
            pos: doc.cur,
            data: Cow::from(spaces.clone()), // Use the calculated spaces
        });
    }

    doc.write_str(&spaces);
    cursor::right(doc, view, n);
}

/// Deletes a character at the current cursor position, joining two lines if necessary.
/// The cursor will be at the delete chars position.
pub fn delete_char(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let cur = doc.cur;

    if cur.x > 0 {
        // If deleting a character in a line.
        cursor::left(doc, view, 1);
        let ch = doc.delete_char(doc.cur.x, doc.cur.y).unwrap();

        if let Some(history) = history {
            history.add_change(Change::Delete {
                pos: doc.cur,
                data: Cow::from(ch.to_string()),
            });
        }
    } else if cur.y > 0 {
        // If deleting at the beginning of a line and it's not the first line.
        cursor::up(doc, view, 1);
        cursor::jump_to_end_of_line(doc, view);

        // Remove line from document.
        let line = doc.remove_line(doc.cur.y + 1).unwrap();

        // Remove newline from previous line.
        let len = doc.buff[cur.y - 1].len();
        doc.buff[cur.y - 1].to_mut().remove(len - 1);

        // Append removed line to previous line.
        doc.buff[cur.y - 1].to_mut().push_str(&line);

        if let Some(history) = history {
            history.add_change(Change::Delete {
                pos: doc.cur,
                data: Cow::from("\n"),
            });
        }
    }
}
