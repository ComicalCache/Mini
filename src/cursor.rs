use crate::document::Document;

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
        Some(self.cmp(other))
    }
}

impl Ord for Cursor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare y coordinates first.
        match self.y.cmp(&other.y) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        // Compare x coordinates second.
        self.x.cmp(&other.x)
    }
}

#[macro_export]
/// Convenience macro for calling movement functions. Expects a `BaseBuffer` as member `base`.
macro_rules! movement {
    ($self:ident, $func:ident) => {{
        $crate::cursor::$func(&mut $self.base.doc, 1);
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
pub fn pos_after_text(start: &Cursor, text: &str) -> Cursor {
    if text.is_empty() {
        return *start;
    }

    let (lines, line_len) = text
        // Use split to not lose empty trailing lines.
        .split('\n')
        .fold((0, 0), |(lines, _), line| (lines + 1, line.chars().count()));

    if lines == 1 {
        // The offset is additive on the same line.
        Cursor::new(start.x + line_len, start.y)
    } else {
        Cursor::new(line_len, start.y + lines - 1)
    }
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

/// Moves the cursors to the right
pub fn right(doc: &mut Document, n: usize) {
    let mut line_bound = doc.line_count(doc.cur.y).unwrap();
    if doc.ends_with_newline(doc.cur.y) {
        line_bound = line_bound.saturating_sub(1);
    }

    doc.cur.right(n, line_bound);
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

/// Jumps the cursors to the next "word".
pub fn next_word(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __next_word(doc);
    }
}

fn __next_word(doc: &mut Document) {
    let end = {
        let y = doc.len().saturating_sub(1);
        let x = doc.line_count(y).unwrap_or(0);
        Cursor::new(x, y)
    };
    if doc.cur == end {
        return;
    }

    let Some(text) = doc.get_range(doc.cur, end) else {
        return;
    };
    let mut chars = text.chars().peekable();
    let mut idx = doc.xy_to_idx(doc.cur.x, doc.cur.y);
    let Some(first) = chars.peek().copied() else {
        return;
    };

    if first.is_alphanumeric() {
        while chars.next_if(|c| c.is_alphanumeric()).is_some() {
            idx += 1;
        }
        while chars.next_if(|c| c.is_whitespace()).is_some() {
            idx += 1;
        }
    } else if first.is_whitespace() {
        while chars.next_if(|c| c.is_whitespace()).is_some() {
            idx += 1;
        }
    } else {
        chars.next();
        idx += 1;
        while chars.next_if(|c| c.is_whitespace()).is_some() {
            idx += 1;
        }
    }

    let (x, y) = doc.idx_to_xy(idx);
    doc.cur = Cursor::new(x, y);
}

/// Jumps the cursors to the end of the next "word".
pub fn next_word_end(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __next_word_end(doc);
    }
}

fn __next_word_end(doc: &mut Document) {
    let end = {
        let end_y = doc.len().saturating_sub(1);
        let end_x = doc.line_count(end_y).unwrap_or(0);
        Cursor::new(end_x, end_y)
    };
    if doc.cur == end {
        return;
    }

    let Some(text) = doc.get_range(doc.cur, end) else {
        return;
    };
    let mut chars = text.chars().peekable();
    let mut idx = doc.xy_to_idx(doc.cur.x, doc.cur.y);
    let Some(first) = chars.peek().copied() else {
        return;
    };

    if first.is_alphanumeric() {
        while chars.next_if(|c| c.is_alphanumeric()).is_some() {
            idx += 1;
        }
    } else if first.is_whitespace() {
        while chars.next_if(|c| c.is_whitespace()).is_some() {
            idx += 1;
        }
        if let Some(c) = chars.peek()
            && !c.is_whitespace()
            && !c.is_alphanumeric()
        {
            idx += 1;
        } else {
            while chars.next_if(|c| c.is_alphanumeric()).is_some() {
                idx += 1;
            }
        }
    } else {
        idx += 1;
    }

    let (x, y) = doc.idx_to_xy(idx);
    doc.cur = Cursor::new(x, y);
}

/// Jumps the cursors to the previous "word".
pub fn prev_word(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __prev_word(doc);
    }
}

fn __prev_word(doc: &mut Document) {
    if doc.cur == Cursor::new(0, 0) {
        return;
    }

    let Some(text) = doc.get_range(Cursor::new(0, 0), doc.cur) else {
        return;
    };
    let mut chars = text.chars_at(text.len_chars()).reversed().peekable();
    let mut idx = doc.xy_to_idx(doc.cur.x, doc.cur.y);
    let Some(first) = chars.peek().copied() else {
        return;
    };

    if first.is_alphanumeric() {
        while chars.next_if(|c| c.is_alphanumeric()).is_some() {
            idx -= 1;
        }
    } else if first.is_whitespace() {
        while chars.next_if(|c| c.is_whitespace()).is_some() {
            idx -= 1;
        }
        if let Some(c) = chars.peek()
            && !c.is_whitespace()
            && !c.is_alphanumeric()
        {
            idx -= 1;
        } else {
            while chars.next_if(|c| c.is_alphanumeric()).is_some() {
                idx -= 1;
            }
        }
    } else {
        idx -= 1;
    }

    let (x, y) = doc.idx_to_xy(idx);
    doc.cur = Cursor::new(x, y);
}

/// Jumps the cursors to the end of the previous "word".
pub fn prev_word_end(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __prev_word_end(doc);
    }
}

fn __prev_word_end(doc: &mut Document) {
    if doc.cur == Cursor::new(0, 0) {
        return;
    }

    let Some(text) = doc.get_range(Cursor::new(0, 0), doc.cur) else {
        return;
    };
    let mut chars = text.chars_at(text.len_chars()).reversed().peekable();
    let mut idx = doc.xy_to_idx(doc.cur.x, doc.cur.y);
    let Some(first) = chars.peek().copied() else {
        return;
    };

    if first.is_alphanumeric() {
        while chars.next_if(|c| c.is_alphanumeric()).is_some() {
            idx -= 1;
        }
        while chars.next_if(|c| c.is_whitespace()).is_some() {
            idx -= 1;
        }
    } else if first.is_whitespace() {
        while chars.next_if(|c| c.is_whitespace()).is_some() {
            idx -= 1;
        }
    } else {
        idx -= 1;
    }

    let (x, y) = doc.idx_to_xy(idx);
    doc.cur = Cursor::new(x, y);
}

/// Jumps to the next whitespace.
pub fn next_whitespace(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __next_whitespace(doc);
    }
}

fn __next_whitespace(doc: &mut Document) {
    let end = {
        let y = doc.len().saturating_sub(1);
        let x = doc.line_count(y).unwrap_or(0);
        Cursor::new(x, y)
    };
    if doc.cur == end {
        return;
    }

    let Some(text) = doc.get_range(doc.cur, end) else {
        return;
    };
    let mut chars = text.chars().peekable();
    let mut idx = doc.xy_to_idx(doc.cur.x, doc.cur.y);

    while chars.next_if(|c| c.is_whitespace()).is_some() {
        idx += 1;
    }
    while chars.next_if(|c| !c.is_whitespace()).is_some() {
        idx += 1;
    }

    let (x, y) = doc.idx_to_xy(idx);
    doc.cur = Cursor::new(x, y);
}

/// Jumps to the previous whitespace.
pub fn prev_whitespace(doc: &mut Document, n: usize) {
    for _ in 0..n {
        __prev_whitespace(doc);
    }
}

fn __prev_whitespace(doc: &mut Document) {
    if doc.cur == Cursor::new(0, 0) {
        return;
    }

    let Some(text) = doc.get_range(Cursor::new(0, 0), doc.cur) else {
        return;
    };
    let mut chars = text.chars_at(text.len_chars()).reversed().peekable();
    let mut idx = doc.xy_to_idx(doc.cur.x, doc.cur.y);

    while chars.next_if(|c| c.is_whitespace()).is_some() {
        idx -= 1;
    }
    while chars.next_if(|c| !c.is_whitespace()).is_some() {
        idx -= 1;
    }
    while chars.next_if(|c| c.is_whitespace()).is_some() {
        idx -= 1;
    }

    let (x, y) = doc.idx_to_xy(idx);
    doc.cur = Cursor::new(x, y);
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
    let Some(current_char) = doc.line(doc.cur.y).unwrap().chars().nth(doc.cur.x) else {
        return None; // Cursor is at the end of line.
    };

    let (opening, closing, forward) = match current_char {
        '(' => ('(', ')', true),
        '[' => ('[', ']', true),
        '{' => ('{', '}', true),
        '<' => ('<', '>', true),
        ')' => (')', '(', false),
        ']' => (']', '[', false),
        '}' => ('}', '{', false),
        '>' => ('>', '<', false),
        _ => return None,
    };

    let end = if forward {
        let end_y = doc.len().saturating_sub(1);
        let end_x = doc.line_count(end_y).unwrap_or(0);
        Cursor::new(end_x, end_y)
    } else {
        Cursor::new(0, 0)
    };

    let text = doc.get_range(doc.cur, end)?;
    let mut chars = if forward {
        text.chars()
    } else {
        text.chars_at(text.len_chars()).reversed()
    };

    // Start with one for backwards search since the initial char is cut off.
    let mut depth = usize::from(!forward);
    let pred = |ch: char| {
        depth += usize::from(ch == opening);
        depth -= usize::from(ch == closing);

        depth == 0
    };
    let offset = chars
        .position(pred)
        // Plus one for backwards search since the last char is cut off.
        .map(|idx| idx + usize::from(!forward))?;

    let idx = doc.xy_to_idx(doc.cur.x, doc.cur.y);
    if forward {
        Some(doc.idx_to_xy(idx + offset))
    } else {
        Some(doc.idx_to_xy(idx - offset))
    }
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
