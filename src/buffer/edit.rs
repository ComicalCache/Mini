use crate::{
    cursor,
    document::Document,
    history::{History, Replace},
    viewport::Viewport,
};

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
        history.add_change(vec![Replace {
            pos: doc.cur,
            delete_data: String::new(),
            insert_data: ch.to_string(),
        }]);
    }

    doc.write_char(ch, doc.cur.x, doc.cur.y);

    if ch == '\n' {
        cursor::down(doc, view, 1);
        cursor::jump_to_beginning_of_line(doc, view);
    } else {
        cursor::right(doc, view, 1);
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
        history.add_change(vec![Replace {
            pos: doc.cur,
            delete_data: String::new(),
            insert_data: spaces.clone(), // Use the calculated spaces
        }]);
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
        let ch = doc.delete_char(doc.cur.x, doc.cur.y);

        if let Some(history) = history {
            history.add_change(vec![Replace {
                pos: doc.cur,
                delete_data: ch.to_string(),
                insert_data: String::new(),
            }]);
        }
    } else if cur.y > 0 {
        // If deleting at the beginning of a line and it's not the first line.
        cursor::up(doc, view, 1);
        cursor::jump_to_end_of_line(doc, view);
        let ch = doc.delete_char(doc.cur.x, doc.cur.y);

        if let Some(history) = history {
            history.add_change(vec![Replace {
                pos: doc.cur,
                delete_data: ch.to_string(),
                insert_data: String::new(),
            }]);
        }
    }
}
