pub struct Cursor {
    pub x: usize,
    pub y: usize,
}

impl Cursor {
    pub fn new(x: usize, y: usize) -> Self {
        Cursor { x, y }
    }

    pub fn left(&mut self, n: usize) {
        self.x = self.x.saturating_sub(n);
    }

    pub fn right(&mut self, n: usize, bound: usize) {
        self.x = (self.x + n).min(bound);
    }

    pub fn up(&mut self, n: usize) {
        self.y = self.y.saturating_sub(n);
    }

    pub fn down(&mut self, n: usize, bound: usize) {
        self.y = (self.y + n).min(bound);
    }
}
