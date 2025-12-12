use crate::{document::Document, viewport::Viewport};

#[derive(Clone, Copy)]
/// The displayed cursor style.
pub enum CursorStyle {
    Hidden,
    SteadyBar,
    SteadyBlock,
}

#[derive(Clone, Copy, Eq)]
/// A cursor position in a document.
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

#[macro_export]
/// Convenience macro for calling movement functions. Expects a `BaseBuffer` as member `base`.
macro_rules! movement {
    ($self:ident, $func:ident) => {{
        $crate::cursor::$func(&mut $self.base.doc, 1);
        $self.base.update_selection();
    }};
    ($self:ident, $func:ident, VIEWPORT) => {{
        $crate::cursor::$func(&mut $self.base.doc, &mut $self.base.doc_view, 1);
        $self.base.update_selection();
    }};
}

#[macro_export]
/// Convenience macro for calling jump functions. Expects a `BaseBuffer` as member `base`.
macro_rules! jump {
    ($self:ident, $func:ident) => {{
        $crate::cursor::$func(&mut $self.base.doc);
        $self.base.update_selection();
    }};
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
pub fn move_to(doc: &mut Document, pos: Cursor) {
    if pos.y < doc.cur.y {
        up(doc, doc.cur.y - pos.y);
    } else if pos.y > doc.cur.y {
        down(doc, pos.y - doc.cur.y);
    }

    if pos.x < doc.cur.x {
        left(doc, doc.cur.x - pos.x);
    } else if pos.x > doc.cur.x {
        right(doc, pos.x - doc.cur.x);
    }
}

/// Moves the cursors to the left.
pub fn left(doc: &mut Document, n: usize) {
    doc.cur.left(n, 0);
}

/// Shifts the viewport to the left.
pub fn viewport_left(doc: &Document, view: &mut Viewport, n: usize) {
    let limit = (doc.cur.x + 1).saturating_sub(view.buff_w);
    view.scroll_x = view.scroll_x.saturating_sub(n).max(limit);
}

/// Moves the cursors to the right
pub fn right(doc: &mut Document, n: usize) {
    let mut line_bound = doc.line_count(doc.cur.y).unwrap();
    if doc.ends_with_newline(doc.cur.y) {
        line_bound = line_bound.saturating_sub(1);
    }

    doc.cur.right(n, line_bound);
}

/// Shifts the viewport to the right.
pub fn viewport_right(doc: &Document, view: &mut Viewport, n: usize) {
    view.scroll_x = (view.scroll_x + n).min(doc.cur.x);
}

/// Moves the cursors up.
pub fn up(doc: &mut Document, n: usize) {
    doc.cur.up(n, 0);

    // When moving up, handle case that new line contains less text than previous.
    let mut line_bound = doc.line_count(doc.cur.y).unwrap();
    if doc.ends_with_newline(doc.cur.y) {
        line_bound = line_bound.saturating_sub(1);
    }

    doc.cur.x = doc.cur.target_x.min(line_bound);
}

/// Shifts the viewport up.
pub fn viewport_up(doc: &Document, view: &mut Viewport, n: usize) {
    view.scroll_y = (view.scroll_y + n).min(doc.cur.y);
}

/// Moves the cursors down.
pub fn down(doc: &mut Document, n: usize) {
    let bound = doc.len().saturating_sub(1);
    doc.cur.down(n, bound);

    // When moving down, handle case that new line contains less text than previous.
    let mut line_bound = doc.line_count(doc.cur.y).unwrap();
    if doc.ends_with_newline(doc.cur.y) {
        line_bound = line_bound.saturating_sub(1);
    }

    doc.cur.x = doc.cur.target_x.min(line_bound);
}

/// Shifts the viewport up.
pub fn viewport_down(doc: &Document, view: &mut Viewport, n: usize) {
    let limit = (doc.cur.y + 1).saturating_sub(view.h);
    view.scroll_y = view.scroll_y.saturating_sub(n).max(limit);
}

/// Jumps the cursors to the next "word".
pub fn next_word(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __next_word(doc);
    }
}

fn __next_word(doc: &mut Document) {
    // Move line down if at end of line and not at end of document.
    let mut len = doc.line_count(doc.cur.y).unwrap();
    if doc.ends_with_newline(doc.cur.y) {
        len = len.saturating_sub(1);
    }

    if len <= doc.cur.x && doc.cur.y < doc.len() {
        jump_to_beginning_of_line(doc);
        down(doc, 1);

        // If empty line or not whitespace, abort.
        if doc
            .line(doc.cur.y)
            .unwrap()
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_whitespace())
        {
            return;
        }

        len = doc.line_count(doc.cur.y).unwrap();
        if doc.ends_with_newline(doc.cur.y) {
            len = len.saturating_sub(1);
        }
    }

    let line = doc.line(doc.cur.y).unwrap();
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
            jump_to_end_of_line(doc);
            return;
        };

        if !ch.is_whitespace() {
            right(doc, jdx + 1);
            return;
        }

        idx = jdx;
    } else if !curr.is_alphanumeric() {
        // If next is not whitespace, move there.
        // Else find next alphanumeric and jump there, else end of line.
        let Some((jdx, ch)) = line.chars().skip(doc.cur.x + 1).enumerate().next() else {
            jump_to_end_of_line(doc);
            return;
        };

        if !ch.is_whitespace() {
            right(doc, jdx + 1);
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
        jump_to_end_of_line(doc);
        return;
    };

    right(doc, idx + jdx + 1);
}

/// Jumps the cursors to the end of the next "word".
pub fn next_word_end(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __next_word_end(doc);
    }
}

fn __next_word_end(doc: &mut Document) {
    // Move line down if at end of line and not at end of document.
    let mut len = doc.line_count(doc.cur.y).unwrap();
    if doc.ends_with_newline(doc.cur.y) {
        len = len.saturating_sub(1);
    }

    if len <= doc.cur.x && doc.cur.y < doc.len() {
        jump_to_beginning_of_line(doc);
        down(doc, 1);

        // If empty line, abort.
        if doc.line(doc.cur.y).unwrap().chars().next().is_none() {
            return;
        }

        len = doc.line_count(doc.cur.y).unwrap();
        if doc.ends_with_newline(doc.cur.y) {
            len = len.saturating_sub(1);
        }
    }

    let line = doc.line(doc.cur.y).unwrap();
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
            jump_to_end_of_line(doc);
            return;
        };

        if !ch.is_alphanumeric() {
            right(doc, jdx + 1);
            return;
        }

        idx = jdx;
    } else if !curr.is_alphanumeric() {
        // If next is not whitespace, move there.
        // Else find next non alphanumeric and jump there, else end of line.
        let Some((jdx, ch)) = line.chars().skip(doc.cur.x + 1).enumerate().next() else {
            jump_to_end_of_line(doc);
            return;
        };

        if !ch.is_whitespace() {
            right(doc, jdx + 1);
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
        jump_to_end_of_line(doc);
        return;
    };

    right(doc, idx + jdx + 1);
}

/// Jumps the cursors to the previous "word".
pub fn prev_word(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __prev_word(doc);
    }
}

fn __prev_word(doc: &mut Document) {
    // Move line up if at beginning of line and not at beginning of document.
    if doc.cur.x == 0 && doc.cur.y > 0 {
        up(doc, 1);
        jump_to_end_of_line(doc);

        // If empty line, abort.
        if doc.line(doc.cur.y).unwrap().len_chars() == 0 {
            return;
        }
    }

    let line = doc.line(doc.cur.y).unwrap();

    // Find next non-whitespace character.
    if let Some((idx, ch)) = line
        .chars_at(doc.cur.x)
        .reversed()
        .enumerate()
        .find(|(_, ch)| !ch.is_whitespace())
    {
        let mut offset = idx + 1;

        // If it's alphanumeric, find first character of that sequence of alphanumeric characters.
        if ch.is_alphanumeric() {
            offset += line
                .chars_at(doc.cur.x - offset)
                .reversed()
                .take_while(|ch| ch.is_alphanumeric())
                .count();
        }

        left(doc, offset);
    } else {
        // Move to the beginning of line.
        jump_to_beginning_of_line(doc);
    }
}

/// Jumps the cursors to the end of the previous "word".
pub fn prev_word_end(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __prev_word_end(doc);
    }
}

fn __prev_word_end(doc: &mut Document) {
    // Move line up if at beginning of line and not at beginning of document.
    if doc.cur.x == 0 && doc.cur.y > 0 {
        up(doc, 1);
        jump_to_end_of_line(doc);

        let line = doc.line(doc.cur.y).unwrap();
        // If empty line or not whitespace, abort.
        if line.len_chars() == 0 || !line.chars().last().unwrap().is_whitespace() {
            return;
        }
    }

    let line = doc.line(doc.cur.y).unwrap();

    // Find next non alphanumeric character.
    if let Some((idx, ch)) = line
        .chars_at(doc.cur.x)
        .reversed()
        .enumerate()
        .find(|(_, ch)| !ch.is_alphanumeric())
    {
        let mut offset = idx;

        // If it's whitespace, find first character of that sequence of alphanumeric characters.
        if ch.is_whitespace() {
            offset += line
                .chars_at(doc.cur.x - offset)
                .reversed()
                .take_while(|ch| ch.is_whitespace())
                .count();
        }

        left(doc, offset.max(1));
    } else {
        // Move to the beginning of line.
        jump_to_beginning_of_line(doc);
    }
}

/// Jumps to the next whitespace.
pub fn next_whitespace(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __next_whitespace(doc);
    }
}

fn __next_whitespace(doc: &mut Document) {
    // Move line down if at end of line and not at end of document.
    let mut len = doc.line_count(doc.cur.y).unwrap();
    if doc.ends_with_newline(doc.cur.y) {
        len = len.saturating_sub(1);
    }

    if len <= doc.cur.x && doc.cur.y < doc.len() {
        jump_to_beginning_of_line(doc);
        down(doc, 1);

        // If empty line or whitespace, abort.
        if doc
            .line(doc.cur.y)
            .unwrap()
            .chars()
            .next()
            .is_none_or(char::is_whitespace)
        {
            return;
        }

        len = doc.line_count(doc.cur.y).unwrap();
        if doc.ends_with_newline(doc.cur.y) {
            len = len.saturating_sub(1);
        }
    }

    let line = doc.line(doc.cur.y).unwrap();

    let (n, _) = line
        .chars()
        .skip(doc.cur.x + 1)
        .enumerate()
        .find(|(_, ch)| ch.is_whitespace())
        .unwrap_or((len - doc.cur.x, '\0'));

    right(doc, n + 1);
}

/// Jumps to the previous whitespace.
pub fn prev_whitespace(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __prev_whitespace(doc);
    }
}

fn __prev_whitespace(doc: &mut Document) {
    // Move line up if at beginning of line and not at beginning of document.
    if doc.cur.x == 0 && doc.cur.y > 0 {
        up(doc, 1);
        jump_to_end_of_line(doc);

        return;
    }

    let line = doc.line(doc.cur.y).unwrap();

    let Some((n, _)) = line
        .chars_at(doc.cur.x)
        .reversed()
        .enumerate()
        .find(|(_, ch)| ch.is_whitespace())
    else {
        // If no whitespace was found on line, move line up, else move to be start of the line.
        if doc.cur.y > 0 {
            up(doc, 1);
            jump_to_end_of_line(doc);
        } else {
            left(doc, doc.cur.x);
        }

        return;
    };

    left(doc, n + 1);
}

/// Jumps to the next empty line.
pub fn next_empty_line(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __next_empty_line(doc);
    }
}

fn __next_empty_line(doc: &mut Document) {
    if let Some((y, _)) = doc
        .lines()
        .enumerate()
        .skip(doc.cur.y + 1)
        .find(|(_, l)| *l == "\n" || l.len_chars() == 0)
    {
        down(doc, y - doc.cur.y);
    } else {
        jump_to_end_of_file(doc);
    }
}

/// Jumps to the previous empty line.
pub fn prev_empty_line(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __prev_empty_line(doc);
    }
}

pub fn __prev_empty_line(doc: &mut Document) {
    for y in (0..doc.cur.y).rev() {
        let line = doc.line(y).unwrap();

        if line.len_chars() == 0 || line == "\n" {
            up(doc, doc.cur.y - y);
            return;
        }
    }

    // If no empty line was found in the loop, jump to the start.
    jump_to_beginning_of_file(doc);
}

/// Jumps the cursors the the beginning of a line.
pub fn jump_to_beginning_of_line(doc: &mut Document) {
    left(doc, doc.cur.x);
}

/// Jumps the cursors to the end of a line.
pub fn jump_to_end_of_line(doc: &mut Document) {
    let mut line_bound = doc.line_count(doc.cur.y).unwrap();
    if doc.ends_with_newline(doc.cur.y) {
        line_bound = line_bound.saturating_sub(1);
    }

    right(doc, line_bound.saturating_sub(doc.cur.x));
}

/// Jumps the cursors to the matching opposite bracket (if exists).
pub fn jump_to_matching_opposite(doc: &mut Document) {
    if let Some((x, y)) = find_matching_bracket(doc) {
        move_to(doc, Cursor::new(x, y));
    }
}

fn find_matching_bracket(doc: &Document) -> Option<(usize, usize)> {
    let cur = doc.cur;
    let Some(current_char) = doc.line(cur.y).unwrap().chars().nth(cur.x) else {
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
        for y in cur.y..doc.len() {
            let line = doc.line(y).unwrap();
            let offset = if y == cur.y { cur.x + 1 } else { 0 };

            for (x, ch) in line.chars().enumerate().skip(offset) {
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
            let line = doc.line(y).unwrap();
            let offset = if y == cur.y { cur.x } else { line.len_chars() };

            for (x, ch) in line.chars_at(offset).reversed().enumerate() {
                if ch == closing {
                    depth += 1;
                } else if ch == opening {
                    depth -= 1;
                }

                if depth == 0 {
                    return Some((offset - x - 1, y));
                }
            }
        }
    }

    None
}

/// Jumps the cursors to the last line of the file.
pub fn jump_to_end_of_file(doc: &mut Document) {
    down(doc, doc.len() - (doc.cur.y + 1));
    jump_to_end_of_line(doc);
}

/// Jumps the cursors to the first line of the file.
pub fn jump_to_beginning_of_file(doc: &mut Document) {
    move_to(doc, Cursor::new(0, 0));
}
