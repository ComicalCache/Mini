pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Command,
    Write,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CursorMove {
    Left,
    Down,
    Up,
    Right,
}

pub struct ScreenDimensions {
    pub w: usize,
    pub h: usize,
}
