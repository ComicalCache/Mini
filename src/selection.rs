use crate::cursor::Cursor;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    Normal,
    Line,
}

/// Represents a selection of text.
#[derive(PartialEq, Eq)]
pub struct Selection {
    pub anchor: Cursor,
    pub head: Cursor,
    pub kind: SelectionKind,

    anchor_line_len: Option<usize>,
    head_line_len: Option<usize>,
}

impl Selection {
    pub const fn new(
        anchor: Cursor,
        head: Cursor,
        kind: SelectionKind,
        anchor_line_len: Option<usize>,
        head_line_len: Option<usize>,
    ) -> Self {
        Self {
            anchor,
            head,
            kind,
            anchor_line_len,
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
        let start = self.anchor.min(self.head);
        let end = self.anchor.max(self.head);

        match self.kind {
            SelectionKind::Normal => (start, end),
            SelectionKind::Line => {
                let start = Cursor::new(0, start.y);

                let end_x = if end == self.head {
                    self.head_line_len.unwrap()
                } else {
                    self.anchor_line_len.unwrap()
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
        let anchor1 = if self.kind == SelectionKind::Line {
            Cursor::new(0, self.anchor.y)
        } else {
            self.anchor
        };
        let anchor2 = if other.kind == SelectionKind::Line {
            Cursor::new(0, other.anchor.y)
        } else {
            other.anchor
        };

        anchor1.cmp(&anchor2)
    }
}
