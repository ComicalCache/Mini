use crate::{
    buffer::history::{Change, History},
    cursor,
    document::Document,
    viewport::Viewport,
};
use std::borrow::Cow;

const TAB: &str = "    ";

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
    cursor::right(doc, view, 1);
}

/// Writes a new line character at the current cursor position.
/// The cursor will be at the beginning of the new line.
pub fn write_new_line_char(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    if let Some(history) = history {
        history.add_change(Change::Insert {
            pos: doc.cur,
            data: Cow::from("\n"),
        });
    }

    let line = &mut doc.buff[doc.cur.y];
    let idx = line
        .char_indices()
        .nth(doc.cur.x)
        .map_or(line.len(), |(idx, _)| idx);

    let new_line = line.to_mut().split_off(idx);
    doc.insert_line(Cow::from(new_line), doc.cur.y + 1);

    cursor::down(doc, view, 1);
    cursor::jump_to_beginning_of_line(doc, view);
}

/// Writes a tab at the current cursor position.
/// The cursor will be after the tab.
pub fn write_tab(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    if let Some(history) = history {
        history.add_change(Change::Insert {
            pos: doc.cur,
            data: Cow::from(TAB),
        });
    }

    doc.write_str(TAB);
    cursor::right(doc, view, TAB.chars().count());
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

        let line = doc.remove_line(doc.cur.y + 1).unwrap();
        doc.buff[cur.y - 1].to_mut().push_str(&line);

        if let Some(history) = history {
            history.add_change(Change::Delete {
                pos: doc.cur,
                data: Cow::from("\n"),
            });
        }
    }
}
