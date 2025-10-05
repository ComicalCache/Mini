use crate::{
    buffer::history::{Change, History},
    cursor::{self, Cursor},
    document::Document,
    viewport::Viewport,
};
use std::borrow::Cow;

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

    cursor::move_to(doc, view, start);

    *sel = None;
}

/// Deletes a line.
pub fn line(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    let end = (doc.cur.y + n).min(doc.buff.len());

    // Only collect the delete data if a history exists.
    let mut data = if history.is_some() {
        Some(doc.buff[doc.cur.y..end].join("\n"))
    } else {
        None
    };

    // Now perform the deletion.
    if !doc.buff.is_empty() {
        doc.buff.drain(doc.cur.y..end);
    }

    if doc.buff.is_empty() {
        // Ensure one line always exists.
        doc.buff.push(Cow::from(""));
    } else if data.is_some() {
        // Append a new line if it was not the only line to be deleted.
        data.as_mut().expect("Illegal state").push('\n');
    }

    if let Some(history) = history
        && !data.as_ref().expect("Illegal state").is_empty()
    {
        history.add_change(Change::Delete {
            pos: Cursor::new(0, doc.cur.y),
            data: Cow::from(data.expect("Illegal state")),
        });
    }

    // Adjust cursor.
    if doc.cur.y >= doc.buff.len() {
        cursor::up(doc, view, doc.cur.y - doc.buff.len());
    }
    let bound = doc.line_count(doc.cur.y).expect("Illegal state");
    if doc.cur.x > bound {
        cursor::left(doc, view, doc.cur.x - bound);
    }

    doc.edited = true;
}

/// Deletes left of the cursor.
pub fn left(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    let mut tmp = Some(doc.cur);
    cursor::left(doc, view, n);
    selection(doc, view, &mut tmp, history);
}

/// Deletes right of the cursor.
pub fn right(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    let mut tmp = Some(doc.cur);
    cursor::right(doc, view, n);
    selection(doc, view, &mut tmp, history);
}

/// Deletes the next word.
pub fn next_word(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    let mut tmp = Some(doc.cur);
    cursor::next_word(doc, view, n);
    selection(doc, view, &mut tmp, history);
}

/// Deletes the previous word.
pub fn prev_word(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    let mut tmp = Some(doc.cur);
    cursor::prev_word(doc, view, n);
    selection(doc, view, &mut tmp, history);
}

/// Deletes until the beginning of the line.
pub fn beginning_of_line(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_beginning_of_line(doc, view);
    selection(doc, view, &mut tmp, history);
}

/// Deletes until the end of the line.
pub fn end_of_line(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_end_of_line(doc, view);
    selection(doc, view, &mut tmp, history);
}

/// Deletes until the matching opposite bracket.
pub fn matching_opposite(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_matching_opposite(doc, view);
    selection(doc, view, &mut tmp, history);
}

/// Deletes until the beginning of the file.
pub fn beginning_of_file(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_beginning_of_file(doc, view);
    selection(doc, view, &mut tmp, history);
}

/// Deletes until the end of the file.
pub fn end_of_file(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_end_of_file(doc, view);
    selection(doc, view, &mut tmp, history);
}
