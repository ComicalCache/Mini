use crate::cursor::Cursor;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    Normal,
    Line,
}

/// Represents a selection of text.
#[derive(PartialEq, Eq)]
pub struct Selection {
    pub start: Cursor,
    pub head: Cursor,
    pub kind: SelectionKind,

    start_line_len: Option<usize>,
    head_line_len: Option<usize>,
}

impl Selection {
    pub const fn new(
        start: Cursor,
        head: Cursor,
        kind: SelectionKind,
        start_line_len: Option<usize>,
        head_line_len: Option<usize>,
    ) -> Self {
        Self {
            start,
            head,
            kind,
            start_line_len,
            head_line_len,
        }
    }

    /// Updates the head of the selection.
    pub const fn update(&mut self, head: Cursor, line_len: Option<usize>) {
        self.head = head;
        self.head_line_len = line_len;
    }

    /// Returns the range of the selection.
    pub fn range(&self) -> (Cursor, Cursor) {
        let (start, end) = if self.start <= self.head {
            (self.start, self.head)
        } else {
            (self.head, self.start)
        };

        match self.kind {
            SelectionKind::Normal => (start, end),
            SelectionKind::Line => {
                let start = Cursor::new(0, start.y);

                let end_x = if end == self.head {
                    self.head_line_len.unwrap()
                } else {
                    self.start_line_len.unwrap()
                };
                let end = Cursor::new(end_x, end.y);

                (start, end)
            }
        }
    }

    /// Checks if a cursor is inside the selection.
    pub fn contains(&self, cur: Cursor) -> bool {
        let (start, end) = self.range();

        match self.kind {
            SelectionKind::Normal => cur >= start && cur < end,
            SelectionKind::Line => cur.y >= start.y && cur.y <= end.y,
        }
    }
}

impl PartialOrd for Selection {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Selection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare y coordinates of start first.
        match self.start.y.cmp(&other.start.y) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        // Compare x coordinates second. In line mode, the effective start x is always 0.
        let self_x = if self.kind == SelectionKind::Line {
            0
        } else {
            self.start.x
        };
        let other_x = if other.kind == SelectionKind::Line {
            0
        } else {
            other.start.x
        };

        self_x.cmp(&other_x)
    }
}
