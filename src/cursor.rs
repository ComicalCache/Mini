use crate::{document::Document, viewport::Viewport};

#[derive(Clone, Copy)]
/// A cursor position in a document or viewport.
pub struct Cursor {
    /// X position.
    pub x: usize,
    /// Target x position when scrolling through lines of varying lengths.
    target_x: usize,
    /// Y position.
    pub y: usize,
}

impl Cursor {
    pub fn new(x: usize, y: usize) -> Self {
        Cursor { y, target_x: x, x }
    }

    /// Moves the cursor to the left.
    fn left(&mut self, n: usize, bound: usize) {
        self.x = self.x.saturating_sub(n).max(bound);
        self.target_x = self.x;
    }

    /// Moves the cursor to the right with a bound.
    fn right(&mut self, n: usize, bound: usize) {
        self.x = (self.x + n).min(bound);
        self.target_x = self.x;
    }

    /// Moves the cursor up.
    fn up(&mut self, n: usize, bound: usize) {
        self.y = self.y.saturating_sub(n).max(bound);
    }

    /// Moves the cursor down with a bound.
    fn down(&mut self, n: usize, bound: usize) {
        self.y = (self.y + n).min(bound);
    }
}

impl PartialEq for Cursor {
    fn eq(&self, other: &Self) -> bool {
        self.y == other.y && self.x == other.x
    }
}

impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Compare y coordinates first.
        match self.y.partial_cmp(&other.y) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }

        // Compare x coordinates second.
        self.x.partial_cmp(&other.x)
    }
}

/// Moves the cursors to the left.
pub fn left(doc: &mut Document, view: &mut Viewport, n: usize) {
    doc.cur.left(n, 0);
    view.cur.left(n, 0);
}

/// Moves the cursors to the right
pub fn right(doc: &mut Document, view: &mut Viewport, n: usize) {
    let line_bound = doc.line_count(doc.cur.y).expect("Illegal state");
    doc.cur.right(n, line_bound);
    view.cur.right(n, (doc.cur.x).min(view.buff_w - 1));
}

/// Moves the cursors up.
pub fn up(doc: &mut Document, view: &mut Viewport, n: usize) {
    doc.cur.up(n, 0);
    // One for info line.
    view.cur.up(n, 0);

    // When moving up, handle case that new line contains less text than previous.
    let line_bound = doc.line_count(doc.cur.y).expect("Illegal state");
    doc.cur.x = doc.cur.target_x.min(line_bound);
    view.cur.x = doc.cur.x.min(view.buff_w - 1);
}

/// Moves the cursors down.
pub fn down(doc: &mut Document, view: &mut Viewport, n: usize) {
    let bound = doc.buff.len().saturating_sub(1);
    doc.cur.down(n, bound);
    // Minus two because one for zero based, one for the info line.
    view.cur.down(n, (view.h - 2).min(bound));

    // When moving down, handle case that new line contains less text than previous.
    let line_bound = doc.line_count(doc.cur.y).expect("Illegal state");
    doc.cur.x = doc.cur.target_x.min(line_bound);
    view.cur.x = doc.cur.x.min(view.buff_w - 1);
}

/// Jumps the cursors to a specific line (or the first/last line if out of bounds).
pub fn jump_to_line(doc: &mut Document, view: &mut Viewport, mut dest: usize) {
    // At most the len of the buffer, at least 1, then subtract one to get the correct index.
    dest = dest.min(doc.buff.len()).max(1) - 1;

    let y = doc.cur.y;
    if dest < y {
        up(doc, view, y - dest);
    } else if dest > y {
        down(doc, view, dest - y);
    }
}

/// Jumps the cursors to the next "word".
pub fn next_word(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __next_word(doc, view);
    }
}

fn __next_word(doc: &mut Document, view: &mut Viewport) {
    let cur = doc.cur;
    let line = &doc.buff[cur.y];
    // Return early if at end of line.
    if line.chars().count() < cur.x {
        return;
    }

    let Some(curr) = line.chars().nth(cur.x) else {
        return;
    };

    // Find next not alphanumeric character or alphanumeric character if the current character is not.
    let Some((idx, ch)) =
        line.chars().skip(cur.x + 1).enumerate().find(|(_, ch)| {
            !ch.is_alphanumeric() || (!curr.is_alphanumeric() && ch.is_alphanumeric())
        })
    else {
        // Jump to end of line if no next word candidate exists.
        jump_to_end_of_line(doc, view);
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

/// Jumps the cursors to the previous "word".
pub fn prev_word(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __prev_word(doc, view);
    }
}

fn __prev_word(doc: &mut Document, view: &mut Viewport) {
    let cur = doc.cur;

    // Return early if already at beginning of line.
    if cur.x == 0 {
        return;
    }

    let line = &doc.buff[cur.y];

    // Find next non-whitespace character.
    if let Some((idx, ch)) = line
        .chars()
        .rev()
        .skip(line.chars().count() - cur.x)
        .enumerate()
        .find(|&(_, ch)| !ch.is_whitespace())
    {
        let mut offset = idx + 1;

        if ch.is_alphanumeric() {
            // If it's alphanumeric, find first character of that sequence of alphanumeric characters.
            offset += line
                .chars()
                .rev()
                .skip(line.chars().count() - cur.x)
                .skip(idx + 1)
                .take_while(|&ch| ch.is_alphanumeric())
                .count();
        }

        left(doc, view, offset);
    } else {
        // Move to the beginning of line.
        jump_to_beginning_of_line(doc, view);
    }
}

/// Jumps the cursors the the beginning of a line.
pub fn jump_to_beginning_of_line(doc: &mut Document, view: &mut Viewport) {
    left(doc, view, doc.cur.x);
}

/// Jumps the cursors to the end of a line.
pub fn jump_to_end_of_line(doc: &mut Document, view: &mut Viewport) {
    right(
        doc,
        view,
        doc.buff[doc.cur.y]
            .chars()
            .count()
            .saturating_sub(doc.cur.x),
    );
}

fn find_matching_bracket(doc: &Document) -> Option<(usize, usize)> {
    let cur = doc.cur;
    let Some(current_char) = doc.buff[cur.y].chars().nth(cur.x) else {
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
        for y in cur.y..doc.buff.len() {
            let line = &doc.buff[y];
            let offset = if y == cur.y { cur.x + 1 } else { 0 };

            for (x, ch) in line.char_indices().skip(offset) {
                if ch == opening {
                    depth += 1;
                } else if ch == closing {
                    depth -= 1;
                }

                if depth == 0 {
                    return Some((x, y));
                }
            }
        }
    } else {
        // Search backward from the character before the cursor.
        for y in (0..=cur.y).rev() {
            let line = &doc.buff[y];
            let offset = if y == cur.y {
                line.chars().count() - cur.x
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
                    return Some((x, y));
                }
            }
        }
    }

    None
}

/// Jumps the cursors to the matching opposite bracket (if exists).
pub fn jump_to_matching_opposite(doc: &mut Document, view: &mut Viewport) {
    let Some((x, y)) = find_matching_bracket(doc) else {
        return;
    };

    if y < doc.cur.y {
        up(doc, view, doc.cur.y - y);
    } else if y > doc.cur.y {
        down(doc, view, y - doc.cur.y);
    }

    if x < doc.cur.x {
        left(doc, view, doc.cur.x - x);
    } else if x > doc.cur.x {
        right(doc, view, x - doc.cur.x);
    }
}

/// Jumps the cursors to the last line of the file.
pub(super) fn jump_to_end_of_file(doc: &mut Document, view: &mut Viewport) {
    down(doc, view, doc.buff.len() - (doc.cur.y + 1));
    left(doc, view, doc.cur.x);
}

/// Jumps the cursors to the first line of the file.
pub(super) fn jump_to_beginning_of_file(doc: &mut Document, view: &mut Viewport) {
    up(doc, view, doc.cur.y + 1);
    left(doc, view, doc.cur.x);
}
