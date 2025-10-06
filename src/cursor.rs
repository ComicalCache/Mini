use crate::{document::Document, viewport::Viewport};
use regex::Regex;
use std::borrow::Cow;

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

/// Calculates the position of a cursor after skipping the supplied text.
pub fn end_pos(start: &Cursor, text: &str) -> Cursor {
    let mut end = *start;

    let mut count = 0;
    let mut offset = 0;
    for line in text.split('\n') {
        count += 1;
        offset = line.chars().count();
    }

    end.y += count - 1;
    if start.y == end.y {
        // The offset is additive on the same line.
        end.x += offset;
    } else {
        end.x = offset;
    }

    end
}

/// Moves the cursor to a specific position.
pub fn move_to(doc: &mut Document, view: &mut Viewport, pos: Cursor) {
    if pos.y < doc.cur.y {
        up(doc, view, doc.cur.y - pos.y);
    } else if pos.y > doc.cur.y {
        down(doc, view, pos.y - doc.cur.y);
    }

    if pos.x < doc.cur.x {
        left(doc, view, doc.cur.x - pos.x);
    } else if pos.x > doc.cur.x {
        right(doc, view, pos.x - doc.cur.x);
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

/// Jumps the cursors to a specific line and column
/// (or the first/last line/character if out of bounds).
pub fn jump_to_line_and_column(
    doc: &mut Document,
    view: &mut Viewport,
    x: Option<usize>,
    y: Option<usize>,
) {
    if let Some(mut y) = y {
        // At most the len of the buffer, at least 1, then subtract one to get the correct index.
        y = y.min(doc.buff.len()).max(1) - 1;

        if y < doc.cur.y {
            up(doc, view, doc.cur.y - y);
        } else if y > doc.cur.y {
            down(doc, view, y - doc.cur.y);
        }
    }

    if let Some(mut x) = x {
        // At most the len of the line, at least 1, then subtract one to get the correct index.
        x = x
            .min(doc.line_count(doc.cur.y).expect("Illegal state"))
            .max(1)
            - 1;

        if x < doc.cur.x {
            left(doc, view, doc.cur.x - x);
        } else if x > doc.cur.x {
            right(doc, view, x - doc.cur.x);
        }
    }
}

/// Jumps the cursors to the next "word".
pub fn next_word(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __next_word(doc, view);
    }
}

fn __next_word(doc: &mut Document, view: &mut Viewport) {
    let cur;
    let line;

    // Move line down if at end of line and not at end of document.
    if doc.buff[doc.cur.y].chars().count() <= doc.cur.x && doc.cur.y < doc.buff.len() - 1 {
        jump_to_beginning_of_line(doc, view);
        down(doc, view, 1);

        // If empty line or not whitespace, abort.
        if doc.buff[doc.cur.y]
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_whitespace())
        {
            return;
        }

        cur = doc.cur;
        line = &doc.buff[cur.y];
    } else {
        cur = doc.cur;
        line = &doc.buff[cur.y];
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
    let cur;
    let line;

    // Move line up if at beginning of line and not at beginning of document.
    if doc.cur.x == 0 && doc.cur.y > 0 {
        up(doc, view, 1);
        jump_to_end_of_line(doc, view);

        // If empty line, abort.
        if doc.buff[doc.cur.y].is_empty() {
            return;
        }

        cur = doc.cur;
        line = &doc.buff[cur.y];
    } else {
        cur = doc.cur;
        line = &doc.buff[cur.y];
    }

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
pub fn jump_to_end_of_file(doc: &mut Document, view: &mut Viewport) {
    down(doc, view, doc.buff.len() - (doc.cur.y + 1));
    left(doc, view, doc.cur.x);
}

/// Jumps the cursors to the first line of the file.
pub fn jump_to_beginning_of_file(doc: &mut Document, view: &mut Viewport) {
    up(doc, view, doc.cur.y + 1);
    left(doc, view, doc.cur.x);
}

/// Finds the start and end cursor positions of a regex search starting at a specific position.
pub fn regex_match(doc: &Document, regex: &Regex) -> Vec<(Cursor, Cursor)> {
    let hay = doc.buff.join("\n");

    // Calculates the cursor based from an offset.
    let cur = |offset: usize, buff: &[Cow<'static, str>]| -> Option<Cursor> {
        let mut count = 0;
        for (y, line) in buff.iter().enumerate() {
            if count + line.len() >= offset {
                let col = line[..offset - count].chars().count();
                return Some(Cursor::new(col, y));
            }

            // Plus one because of newline.
            count += line.len() + 1;
        }

        if offset == count {
            let y = buff.len().saturating_sub(1);
            let x = if let Some(line) = buff.get(y) {
                line.chars().count()
            } else {
                0
            };
            return Some(Cursor::new(x, y));
        }

        None
    };

    regex
        .find_iter(&hay)
        .filter_map(|mat| Some((cur(mat.start(), &doc.buff)?, cur(mat.end(), &doc.buff)?)))
        .collect()
}
