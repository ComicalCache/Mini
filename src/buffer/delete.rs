use crate::{
    cursor::{self, Cursor},
    document::Document,
    viewport::Viewport,
};
use std::borrow::Cow;

/// Deletes the selected area.
pub fn selection(doc: &mut Document, view: &mut Viewport, selection: &mut Option<Cursor>) {
    let Some(pos) = *selection else {
        return;
    };

    let cur = doc.cur;
    let (start, end) = if pos <= cur { (pos, cur) } else { (cur, pos) };

    if start.y == end.y {
        let line = &mut doc.buff[start.y];
        let start_idx = line
            .char_indices()
            .nth(start.x)
            .map_or(line.len(), |(idx, _)| idx);
        let end_idx = line
            .char_indices()
            .nth(end.x)
            .map_or(line.len(), |(idx, _)| idx);

        line.to_mut().drain(start_idx..end_idx);
    } else {
        let (start_line, remaining_lines) = doc.buff.split_at_mut(start.y + 1);
        let end_line = &remaining_lines[end.y - (start.y + 1)];
        let tail = {
            let start_idx = end_line
                .char_indices()
                .nth(end.x)
                .map_or(end_line.len(), |(idx, _)| idx);
            &end_line[start_idx..]
        };

        let start_idx = start_line[start.y]
            .char_indices()
            .nth(start.x)
            .map_or(start_line[start.y].len(), |(idx, _)| idx);
        start_line[start.y].to_mut().truncate(start_idx);
        start_line[start.y].to_mut().push_str(tail);

        // Remove the inbetween lines
        doc.buff.drain((start.y + 1)..=end.y);
    }

    // Allign the cursor
    if cur.y > start.y {
        let diff = cur.y - start.y;
        cursor::up(doc, view, diff);
    }
    if cur.x > start.x {
        let diff = cur.x - start.x;
        cursor::left(doc, view, diff);
    } else if cur.x < start.x {
        let diff = start.x - cur.x;
        cursor::right(doc, view, diff);
    }

    *selection = None;
    doc.edited = true;
}

/// Deletes a line.
pub fn line(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        cursor::jump_to_beginning_of_line(doc, view);
        doc.remove_line();
        if doc.buff.is_empty() {
            doc.insert_line(Cow::from(""));
        }
        if doc.cur.y == doc.buff.len() {
            cursor::up(doc, view, 1);
        }
    }
}

/// Deletes left of the cursor.
pub fn left(doc: &mut Document, view: &mut Viewport, n: usize) {
    let mut tmp = Some(doc.cur);
    cursor::left(doc, view, n);
    selection(doc, view, &mut tmp);
}

/// Deletes right of the cursor.
pub fn right(doc: &mut Document, view: &mut Viewport, n: usize) {
    let mut tmp = Some(doc.cur);
    cursor::right(doc, view, n);
    selection(doc, view, &mut tmp);
}

/// Deletes the next word.
pub fn next_word(doc: &mut Document, view: &mut Viewport, n: usize) {
    let mut tmp = Some(doc.cur);
    cursor::next_word(doc, view, n);
    selection(doc, view, &mut tmp);
}

/// Deletes the previous word.
pub fn prev_word(doc: &mut Document, view: &mut Viewport, n: usize) {
    let mut tmp = Some(doc.cur);
    cursor::prev_word(doc, view, n);
    selection(doc, view, &mut tmp);
}

/// Deletes until the beginning of the line.
pub fn beginning_of_line(doc: &mut Document, view: &mut Viewport) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_beginning_of_line(doc, view);
    selection(doc, view, &mut tmp);
}

/// Deletes until the end of the line.
pub fn end_of_line(doc: &mut Document, view: &mut Viewport) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_end_of_line(doc, view);
    selection(doc, view, &mut tmp);
}

/// Deletes until the matching opposite bracket.
pub fn matching_opposite(doc: &mut Document, view: &mut Viewport) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_matching_opposite(doc, view);
    selection(doc, view, &mut tmp);
}

/// Deletes until the beginning of the file.
pub fn beginning_of_file(doc: &mut Document, view: &mut Viewport) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_beginning_of_file(doc, view);
    selection(doc, view, &mut tmp);
}

/// Deletes until the end of the file.
pub fn end_of_file(doc: &mut Document, view: &mut Viewport) {
    let mut tmp = Some(doc.cur);
    cursor::jump_to_end_of_file(doc, view);
    selection(doc, view, &mut tmp);
}
