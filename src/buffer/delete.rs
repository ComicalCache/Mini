use crate::{
    buffer::history::{Change, History},
    cursor::{self, Cursor},
    document::Document,
    viewport::Viewport,
};

/// Deletes the selected area.
pub fn selection(
    doc: &mut Document,
    view: &mut Viewport,
    sel: &mut Option<Cursor>,
    history: Option<&mut History>,
) {
    let Some(pos) = *sel else {
        return;
    };

    let cur = doc.cur;
    let (start, end) = if pos <= cur { (pos, cur) } else { (cur, pos) };

    if let Some(history) = history
        && let Some(data) = doc.get_range(start, end)
    {
        history.add_change(Change::Delete { pos: start, data });
    }
    doc.remove_range(start, end);

    // Place cursor at the beginning of the deleted area.
    cursor::move_to(doc, view, start);

    *sel = None;
}

/// Deletes a line.
pub fn line(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    if doc.buff.len() == 1 && doc.buff[0].is_empty() {
        return;
    }

    if doc.cur.y + n >= doc.buff.len() {
        cursor::up(doc, view, doc.cur.y + n - doc.buff.len());
    }

    // Begin of selection at the end of one line above the first line or at beginning of current line
    // if in the first line.
    cursor::up(doc, view, 1);
    if doc.cur.y != 0 {
        cursor::jump_to_end_of_line(doc, view);
    } else {
        cursor::jump_to_beginning_of_line(doc, view);
    }
    let tmp = doc.cur;

    // End selection at the end of the last line or at the beginning of the next line if selection started
    // in the first line.
    cursor::down(doc, view, n);
    if tmp.y != 0 {
        cursor::jump_to_end_of_line(doc, view);
    } else {
        cursor::jump_to_beginning_of_line(doc, view);
    }

    selection(doc, view, &mut Some(tmp), history);

    // Fix cursor moving up due to moving it one line up.
    if tmp.y != 0 {
        cursor::down(doc, view, 1);
    }
    cursor::jump_to_beginning_of_line(doc, view);
}

/// Deletes a character.
pub fn char(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    // Move left so that all n - 1 characters can be deleted. Minus one because deleting the place
    // after the line causes it to move left by one. This all aids usability, otherwise it would be
    // identical to the right deletion, however this is a more expected behaviour.
    let len = doc.line_count(doc.cur.y).expect("Illegal state");
    if doc.cur.x + n > len {
        cursor::left(doc, view, doc.cur.x + n - len - 1);
    }

    right(doc, view, history, n);

    // Move the cursor one to the left so it can't end up after the last character after deletion.
    if doc.cur.x == doc.line_count(doc.cur.y).expect("Illegal state") {
        cursor::left(doc, view, 1);
    }
}

/// Deletes left of the cursor.
pub fn left(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    let tmp = doc.cur;
    cursor::left(doc, view, n);
    selection(doc, view, &mut Some(tmp), history);
}

/// Deletes right of the cursor.
pub fn right(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    let tmp = doc.cur;
    cursor::right(doc, view, n);
    selection(doc, view, &mut Some(tmp), history);
}

/// Deletes the next word.
pub fn next_word(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    let tmp = doc.cur;
    cursor::next_word(doc, view, n);
    selection(doc, view, &mut Some(tmp), history);
}

/// Deletes the previous word.
pub fn prev_word(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    let tmp = doc.cur;
    cursor::prev_word(doc, view, n);
    selection(doc, view, &mut Some(tmp), history);
}

/// Deletes until the beginning of the line.
pub fn beginning_of_line(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let tmp = doc.cur;
    cursor::jump_to_beginning_of_line(doc, view);
    selection(doc, view, &mut Some(tmp), history);
}

/// Deletes until the end of the line.
pub fn end_of_line(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let tmp = doc.cur;
    cursor::jump_to_end_of_line(doc, view);
    selection(doc, view, &mut Some(tmp), history);
}

/// Deletes until the matching opposite bracket.
pub fn matching_opposite(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let tmp = doc.cur;
    cursor::jump_to_matching_opposite(doc, view);
    selection(doc, view, &mut Some(tmp), history);
}

/// Deletes until the beginning of the file.
pub fn beginning_of_file(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let tmp = doc.cur;
    cursor::jump_to_beginning_of_file(doc, view);
    selection(doc, view, &mut Some(tmp), history);
}

/// Deletes until the end of the file.
pub fn end_of_file(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let tmp = doc.cur;
    cursor::jump_to_end_of_file(doc, view);
    selection(doc, view, &mut Some(tmp), history);
}
