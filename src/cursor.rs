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
    pub const fn new(x: usize, y: usize) -> Self {
        Self { y, target_x: x, x }
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

/// Shifts the viewport to the left.
// Allow unused mut to aid macro usage in src/buffer/base.rs.
#[allow(clippy::needless_pass_by_ref_mut)]
pub fn shift_left(_: &mut Document, view: &mut Viewport, n: usize) {
    view.cur.left(n, 0);
}

/// Moves the cursors to the right
pub fn right(doc: &mut Document, view: &mut Viewport, n: usize) {
    let mut line_bound = doc.line_count(doc.cur.y).unwrap();
    if doc.buff[doc.cur.y].ends_with('\n') {
        line_bound = line_bound.saturating_sub(1);
    }

    doc.cur.right(n, line_bound);
    view.cur.right(n, (doc.cur.x).min(view.buff_w - 1));
}

/// Shifts the viewport to the right.
// Allow unused mut to aid macro usage in src/buffer/base.rs.
#[allow(clippy::needless_pass_by_ref_mut)]
pub fn shift_right(doc: &mut Document, view: &mut Viewport, n: usize) {
    view.cur.right(n, doc.cur.x.min(view.buff_w - 1));
}

/// Moves the cursors up.
pub fn up(doc: &mut Document, view: &mut Viewport, n: usize) {
    doc.cur.up(n, 0);
    view.cur.up(n, 0);

    // When moving up, handle case that new line contains less text than previous.
    let mut line_bound = doc.line_count(doc.cur.y).unwrap();
    if doc.buff[doc.cur.y].ends_with('\n') {
        line_bound = line_bound.saturating_sub(1);
    }

    doc.cur.x = doc.cur.target_x.min(line_bound);
    view.cur.x = doc.cur.x.min(view.buff_w - 1);
}

/// Shifts the viewport up.
// Allow unused mut to aid macro usage in src/buffer/base.rs.
#[allow(clippy::needless_pass_by_ref_mut)]
pub fn shift_up(_: &mut Document, view: &mut Viewport, n: usize) {
    view.cur.up(n, 0);
}

/// Moves the cursors down.
pub fn down(doc: &mut Document, view: &mut Viewport, n: usize) {
    let bound = doc.buff.len().saturating_sub(1);
    doc.cur.down(n, bound);
    // Minus one because zero based.
    view.cur.down(n, (view.h - 1).min(bound));

    // When moving down, handle case that new line contains less text than previous.
    let mut line_bound = doc.line_count(doc.cur.y).unwrap();
    if doc.buff[doc.cur.y].ends_with('\n') {
        line_bound = line_bound.saturating_sub(1);
    }

    doc.cur.x = doc.cur.target_x.min(line_bound);
    view.cur.x = doc.cur.x.min(view.buff_w - 1);
}

/// Shifts the viewport up.
// Allow unused mut to aid macro usage in src/buffer/base.rs.
#[allow(clippy::needless_pass_by_ref_mut)]
pub fn shift_down(doc: &mut Document, view: &mut Viewport, n: usize) {
    view.cur.down(n, doc.cur.y.min(view.h - 1));
}

/// Jumps the cursors to the next "word".
pub fn next_word(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __next_word(doc, view);
    }
}

fn __next_word(doc: &mut Document, view: &mut Viewport) {
    // Move line down if at end of line and not at end of document.
    let mut len = doc.line_count(doc.cur.y).unwrap();
    if doc.buff[doc.cur.y].ends_with('\n') {
        len = len.saturating_sub(1);
    }

    if len <= doc.cur.x && doc.cur.y < doc.buff.len() {
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

        len = doc.line_count(doc.cur.y).unwrap();
        if doc.buff[doc.cur.y].ends_with('\n') {
            len = len.saturating_sub(1);
        }
    }

    let line = &doc.buff[doc.cur.y];
    let curr = line
        .chars()
        .nth(doc.cur.x.min(len.saturating_sub(1)))
        .unwrap();
    let mut idx = 0;

    if curr.is_alphanumeric() {
        // Find next non alphanumeric:
        // - if non whitespace, jump there,
        // - if whitespace, find next alphanumeric, jump there,
        // - else jump to end of line.
        let Some((jdx, ch)) = line
            .chars()
            .skip(doc.cur.x + 1)
            .enumerate()
            .find(|(_, ch)| !ch.is_alphanumeric())
        else {
            jump_to_end_of_line(doc, view);
            return;
        };

        if !ch.is_whitespace() {
            right(doc, view, jdx + 1);
            return;
        }

        idx = jdx;
    } else if !curr.is_alphanumeric() {
        // If next is not whitespace, move there.
        // Else find next alphanumeric and jump there, else end of line.
        let Some((jdx, ch)) = line.chars().skip(doc.cur.x + 1).enumerate().next() else {
            jump_to_end_of_line(doc, view);
            return;
        };

        if !ch.is_whitespace() {
            right(doc, view, jdx + 1);
            return;
        }

        idx = jdx;
    }

    // Find next alphanumeric and jump there, else end of line.
    let Some((jdx, _)) = line
        .chars()
        .skip(doc.cur.x + 1 + idx)
        .enumerate()
        .find(|(_, ch)| !ch.is_whitespace())
    else {
        jump_to_end_of_line(doc, view);
        return;
    };

    right(doc, view, idx + jdx + 1);
}

/// Jumps the cursors to the end of the next "word".
pub fn next_word_end(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __next_word_end(doc, view);
    }
}

fn __next_word_end(doc: &mut Document, view: &mut Viewport) {
    // Move line down if at end of line and not at end of document.
    let mut len = doc.line_count(doc.cur.y).unwrap();
    if doc.buff[doc.cur.y].ends_with('\n') {
        len = len.saturating_sub(1);
    }

    if len <= doc.cur.x && doc.cur.y < doc.buff.len() {
        jump_to_beginning_of_line(doc, view);
        down(doc, view, 1);

        // If empty line, abort.
        if doc.buff[doc.cur.y].chars().next().is_none() {
            return;
        }

        len = doc.line_count(doc.cur.y).unwrap();
        if doc.buff[doc.cur.y].ends_with('\n') {
            len = len.saturating_sub(1);
        }
    }

    let line = &doc.buff[doc.cur.y];
    let curr = line
        .chars()
        .nth(doc.cur.x.min(len.saturating_sub(1)))
        .unwrap();
    let mut idx = 0;

    if curr.is_whitespace() {
        // Find next non whitespace:
        // - if alphanumeric, find next non alphanumeric and jump there, else end of line,
        // - if not alphanumeric, jump there,
        // - else end of line.
        let Some((jdx, ch)) = line
            .chars()
            .skip(doc.cur.x + 1)
            .enumerate()
            .find(|(_, ch)| !ch.is_whitespace())
        else {
            jump_to_end_of_line(doc, view);
            return;
        };

        if !ch.is_alphanumeric() {
            right(doc, view, jdx + 1);
            return;
        }

        idx = jdx;
    } else if !curr.is_alphanumeric() {
        // If next is not whitespace, move there.
        // Else find next non alphanumeric and jump there, else end of line.
        let Some((jdx, ch)) = line.chars().skip(doc.cur.x + 1).enumerate().next() else {
            jump_to_end_of_line(doc, view);
            return;
        };

        if !ch.is_whitespace() {
            right(doc, view, jdx + 1);
            return;
        }

        idx = jdx;
    }

    // Find next non alphanumeric and jump there, else to end of line.
    let Some((jdx, _)) = line
        .chars()
        .skip(doc.cur.x + 1 + idx)
        .enumerate()
        .find(|(_, ch)| !ch.is_alphanumeric())
    else {
        jump_to_end_of_line(doc, view);
        return;
    };

    right(doc, view, idx + jdx + 1);
}

/// Jumps the cursors to the previous "word".
pub fn prev_word(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __prev_word(doc, view);
    }
}

fn __prev_word(doc: &mut Document, view: &mut Viewport) {
    // Move line up if at beginning of line and not at beginning of document.
    if doc.cur.x == 0 && doc.cur.y > 0 {
        up(doc, view, 1);
        jump_to_end_of_line(doc, view);

        // If empty line, abort.
        if doc.buff[doc.cur.y].is_empty() {
            return;
        }
    }

    let len = doc.line_count(doc.cur.y).unwrap();
    let line = &doc.buff[doc.cur.y];

    // Find next non-whitespace character.
    if let Some((idx, ch)) = line
        .chars()
        .rev()
        .skip(len - doc.cur.x)
        .enumerate()
        .find(|(_, ch)| !ch.is_whitespace())
    {
        let mut offset = idx + 1;

        // If it's alphanumeric, find first character of that sequence of alphanumeric characters.
        if ch.is_alphanumeric() {
            offset += line
                .chars()
                .rev()
                .skip(len - doc.cur.x + offset)
                .take_while(|ch| ch.is_alphanumeric())
                .count();
        }

        left(doc, view, offset);
    } else {
        // Move to the beginning of line.
        jump_to_beginning_of_line(doc, view);
    }
}

/// Jumps the cursors to the end of the previous "word".
pub fn prev_word_end(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __prev_word_end(doc, view);
    }
}

fn __prev_word_end(doc: &mut Document, view: &mut Viewport) {
    // Move line up if at beginning of line and not at beginning of document.
    if doc.cur.x == 0 && doc.cur.y > 0 {
        up(doc, view, 1);
        jump_to_end_of_line(doc, view);

        // If empty line or not whitespace, abort.
        if doc.buff[doc.cur.y].is_empty()
            || !doc.buff[doc.cur.y].chars().last().unwrap().is_whitespace()
        {
            return;
        }
    }

    let len = doc.line_count(doc.cur.y).unwrap();
    let line = &doc.buff[doc.cur.y];

    // Find next non alphanumeric character.
    if let Some((idx, ch)) = line
        .chars()
        .rev()
        .skip(len - doc.cur.x)
        .enumerate()
        .find(|(_, ch)| !ch.is_alphanumeric())
    {
        let mut offset = idx;

        // If it's whitespace, find first character of that sequence of alphanumeric characters.
        if ch.is_whitespace() {
            offset += line
                .chars()
                .rev()
                .skip(len - doc.cur.x + offset)
                .take_while(|ch| ch.is_whitespace())
                .count();
        }

        left(doc, view, offset.max(1));
    } else {
        // Move to the beginning of line.
        jump_to_beginning_of_line(doc, view);
    }
}

/// Jumps to the next whitespace.
pub fn next_whitespace(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __next_whitespace(doc, view);
    }
}

fn __next_whitespace(doc: &mut Document, view: &mut Viewport) {
    // Move line down if at end of line and not at end of document.
    let mut len = doc.line_count(doc.cur.y).unwrap();
    if doc.buff[doc.cur.y].ends_with('\n') {
        len = len.saturating_sub(1);
    }

    if len <= doc.cur.x && doc.cur.y < doc.buff.len() {
        jump_to_beginning_of_line(doc, view);
        down(doc, view, 1);

        // If empty line or whitespace, abort.
        if doc.buff[doc.cur.y]
            .chars()
            .next()
            .is_none_or(char::is_whitespace)
        {
            return;
        }

        len = doc.line_count(doc.cur.y).unwrap();
        if doc.buff[doc.cur.y].ends_with('\n') {
            len = len.saturating_sub(1);
        }
    }

    let line = &doc.buff[doc.cur.y];

    let (n, _) = line
        .chars()
        .skip(doc.cur.x + 1)
        .enumerate()
        .find(|(_, ch)| ch.is_whitespace())
        .unwrap_or((len - doc.cur.x, '\0'));

    right(doc, view, n + 1);
}

/// Jumps to the previous whitespace.
pub fn prev_whitespace(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __prev_whitespace(doc, view);
    }
}

fn __prev_whitespace(doc: &mut Document, view: &mut Viewport) {
    // Move line up if at beginning of line and not at beginning of document.
    if doc.cur.x == 0 && doc.cur.y > 0 {
        up(doc, view, 1);
        jump_to_end_of_line(doc, view);

        return;
    }

    let len = doc.line_count(doc.cur.y).unwrap();
    let line = &doc.buff[doc.cur.y];

    let Some((n, _)) = line
        .chars()
        .rev()
        .skip(len - doc.cur.x)
        .enumerate()
        .find(|(_, ch)| ch.is_whitespace())
    else {
        // If no whitespace was found on line, move line up, else move to be start of the line.
        if doc.cur.y > 0 {
            up(doc, view, 1);
            jump_to_end_of_line(doc, view);
        } else {
            left(doc, view, doc.cur.x);
        }

        return;
    };

    left(doc, view, n + 1);
}

/// Jumps to the next empty line.
pub fn next_empty_line(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __next_empty_line(doc, view);
    }
}

fn __next_empty_line(doc: &mut Document, view: &mut Viewport) {
    if let Some((y, _)) = doc
        .buff
        .iter()
        .enumerate()
        .skip(doc.cur.y + 1)
        .find(|(_, l)| *l == "\n" || l.is_empty())
    {
        down(doc, view, y - doc.cur.y);
    } else {
        jump_to_end_of_file(doc, view);
    }
}

/// Jumps to the previous empty line.
pub fn prev_empty_line(doc: &mut Document, view: &mut Viewport, n: usize) {
    for _ in 0..n {
        __prev_empty_line(doc, view);
    }
}

pub fn __prev_empty_line(doc: &mut Document, view: &mut Viewport) {
    if let Some((y, _)) = doc
        .buff
        .iter()
        .enumerate()
        .rev()
        .skip(doc.buff.len() - doc.cur.y)
        .find(|(_, l)| *l == "\n" || l.is_empty())
    {
        up(doc, view, doc.cur.y - y);
    } else {
        jump_to_beginning_of_file(doc, view);
    }
}

/// Jumps the cursors the the beginning of a line.
pub fn jump_to_beginning_of_line(doc: &mut Document, view: &mut Viewport) {
    left(doc, view, doc.cur.x);
}

/// Jumps the cursors to the end of a line.
pub fn jump_to_end_of_line(doc: &mut Document, view: &mut Viewport) {
    let mut line_bound = doc.line_count(doc.cur.y).unwrap();
    if doc.buff[doc.cur.y].ends_with('\n') {
        line_bound = line_bound.saturating_sub(1);
    }

    right(doc, view, line_bound.saturating_sub(doc.cur.x));
}

/// Jumps the cursors to the matching opposite bracket (if exists).
pub fn jump_to_matching_opposite(doc: &mut Document, view: &mut Viewport) {
    if let Some((x, y)) = find_matching_bracket(doc) {
        move_to(doc, view, Cursor::new(x, y));
    }
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

/// Jumps the cursors to the last line of the file.
pub fn jump_to_end_of_file(doc: &mut Document, view: &mut Viewport) {
    down(doc, view, doc.buff.len() - (doc.cur.y + 1));
    jump_to_end_of_line(doc, view);
}

/// Jumps the cursors to the first line of the file.
pub fn jump_to_beginning_of_file(doc: &mut Document, view: &mut Viewport) {
    move_to(doc, view, Cursor::new(0, 0));
}
