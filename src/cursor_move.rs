use crate::{document::Document, viewport::Viewport};

/// Moves the cursor to the left.
pub fn left(doc: &mut Document, view: &mut Viewport, n: usize) {
    doc.cur.left(n);
    view.cur.left(n);
}

/// Moves the cursor to the right
pub fn right(doc: &mut Document, view: &mut Viewport, n: usize) {
    let line_bound = doc.line_count(doc.cur.y);
    doc.cur.right(n, line_bound);
    view.cur.right(n, line_bound.min(view.w - 1));
}

/// Moves the cursor up.
pub fn up(doc: &mut Document, view: &mut Viewport, n: usize) {
    doc.cur.up(n);

    // When moving up, handle case that new line contains less text than previous.
    let line_bound = doc.line_count(doc.cur.y);
    if doc.cur.x >= line_bound {
        let diff = doc.cur.x - line_bound;
        doc.cur.left(diff);
        view.cur.left(diff);
    }
}

/// Moves the cursor down.
pub fn down(doc: &mut Document, view: &mut Viewport, n: usize) {
    let bound = doc.buff.len().saturating_sub(1);
    doc.cur.down(n, bound);

    // When moving down, handle case that new line contains less text than previous.
    let line_bound = doc.line_count(doc.cur.y);
    if doc.cur.x >= line_bound {
        let diff = doc.cur.x - line_bound;
        doc.cur.left(diff);
        view.cur.left(diff);
    }
}

/// Jumps to the next "word".
pub fn next_word(doc: &mut Document, view: &mut Viewport) {
    let cursor = doc.cur;
    let line = &doc.buff[cursor.y];
    // Return early if at end of line.
    if line.chars().count() <= cursor.x + 1 {
        return;
    }

    let Some(curr) = line.chars().nth(cursor.x) else {
        return;
    };

    // Find next not alphanumeric character or alphanumeric character if the current character is not.
    let Some((idx, ch)) =
        line.chars().skip(cursor.x + 1).enumerate().find(|(_, ch)| {
            !ch.is_alphanumeric() || (!curr.is_alphanumeric() && ch.is_alphanumeric())
        })
    else {
        // Return early if no next "word" candidate exists.
        return;
    };

    if ch.is_whitespace() {
        // Find next non-whitespace after whitespace.
        let Some((jdx, _)) = line
            .chars()
            .skip(view.cur.x + 1 + idx)
            .enumerate()
            .find(|(_, ch)| !ch.is_whitespace())
        else {
            // Return early if after the whitespace there are no alphanumeric characters.
            return;
        };

        // Move the cursor to the next "word",
        right(doc, view, idx + jdx + 1);
    } else {
        // If it is not whitespace set cursor to the position of the character.
        right(doc, view, idx + 1);
    }
}

/// Jumps to the previous "word".
pub fn prev_word(doc: &mut Document, view: &mut Viewport) {
    let cursor = doc.cur;

    // Return early if already at beginning of line.
    if cursor.x == 0 {
        return;
    }

    let line = &doc.buff[cursor.y];

    // Find next non-whitespace character.
    if let Some((idx, ch)) = line
        .chars()
        .rev()
        .skip(line.chars().count() - cursor.x)
        .enumerate()
        .find(|&(_, ch)| !ch.is_whitespace())
    {
        let mut offset = idx + 1;

        if ch.is_alphanumeric() {
            // If it's alphanumeric, find first character of that sequence of alphanumeric characters.
            offset += line
                .chars()
                .rev()
                .skip(line.chars().count() - cursor.x)
                .skip(idx + 1)
                .take_while(|&ch| ch.is_alphanumeric())
                .count();
        }

        left(doc, view, offset);
    } else {
        // Move to the beginning of line.
        left(doc, view, cursor.x);
    }
}

/// Jumps the the beginning of a line.
pub fn jump_to_beginning_of_line(doc: &mut Document, view: &mut Viewport) {
    left(doc, view, doc.cur.x);
}

/// Jumps to the end of a line.
pub fn jump_to_end_of_line(doc: &mut Document, view: &mut Viewport) {
    right(
        doc,
        view,
        doc.buff[doc.cur.y]
            .chars()
            .count()
            .saturating_sub(doc.cur.x + 1),
    );
}

fn find_matching_bracket(doc: &Document) -> Option<(usize, usize)> {
    let cursor = doc.cur;
    let Some(current_char) = doc.buff[cursor.y].chars().nth(cursor.x) else {
        return None; // Cursor is at the end of line.
    };

    let (opening, closing, forward) = match current_char {
        '(' => ('(', ')', true),
        '[' => ('[', ']', true),
        '{' => ('{', '}', true),
        '<' => ('<', '>', true),
        ')' => ('(', ')', false),
        ']' => ('[', ']', false),
        '}' => ('{', '}', false),
        '>' => ('<', '>', false),
        _ => return None,
    };

    let mut depth = 1;
    if forward {
        // Search forward from the character after the cursor.
        for y in cursor.y..doc.buff.len() {
            let line = &doc.buff[y];
            let offset = if y == cursor.y { cursor.x + 1 } else { 0 };

            for (x, ch) in line.char_indices().skip(offset) {
                if ch == opening {
                    depth += 1;
                } else if ch == closing {
                    depth -= 1;
                }

                if depth == 0 {
                    return Some((y, x));
                }
            }
        }
    } else {
        // Search backward from the character before the cursor.
        for y in (0..=cursor.y).rev() {
            let line = &doc.buff[y];
            let offset = if y == cursor.y {
                line.chars().count() - cursor.x
            } else {
                0
            };

            for (x, ch) in line.char_indices().rev().skip(offset) {
                if ch == closing {
                    depth += 1;
                } else if ch == opening {
                    depth -= 1;
                }

                if depth == 0 {
                    return Some((y, x));
                }
            }
        }
    }

    None
}

/// Jumps to the matching opposite bracket (if exists).
pub fn jump_to_matching_opposite(doc: &mut Document, view: &mut Viewport) {
    let cursor = doc.cur;
    let Some((y, x)) = find_matching_bracket(doc) else {
        return;
    };

    if y < cursor.y {
        up(doc, view, cursor.y - y);
    } else if y > cursor.y {
        down(doc, view, y - cursor.y);
    }

    if x < cursor.x {
        left(doc, view, cursor.x - x);
    } else if x > cursor.x {
        right(doc, view, x - cursor.x);
    }
}

/// Jumps to the last line of the file.
pub(super) fn jump_to_end_of_file(doc: &mut Document, view: &mut Viewport) {
    down(doc, view, doc.buff.len() - (doc.cur.y + 1));
    left(doc, view, doc.cur.x);
}

/// Jumps to the first line of the file.
pub(super) fn jump_to_beginning_of_file(doc: &mut Document, view: &mut Viewport) {
    up(doc, view, doc.cur.y + 1);
    left(doc, view, doc.cur.x);
}
