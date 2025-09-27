#[derive(Clone, Copy)]
pub struct Cursor {
    pub x: usize,
    pub y: usize,
}

impl Cursor {
    pub fn new(x: usize, y: usize) -> Self {
        Cursor { x, y }
    }

    /// Moves the cursor to the left.
    pub fn left(&mut self, n: usize) {
        self.x = self.x.saturating_sub(n);
    }

    /// Moves the cursor to the right with a bound.
    pub fn right(&mut self, n: usize, bound: usize) {
        self.x = (self.x + n).min(bound);
    }

    /// Moves the cursor up.
    pub fn up(&mut self, n: usize) {
        self.y = self.y.saturating_sub(n);
    }

    /// Moves the cursor down with a bound.
    pub fn down(&mut self, n: usize, bound: usize) {
        self.y = (self.y + n).min(bound);
    }
}
