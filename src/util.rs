pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    View,
    Write,
    Command,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CursorMove {
    Left,
    Down,
    Up,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CmdResult {
    Quit,
    Continue,
}

pub struct ScreenDimensions {
    pub w: usize,
    pub h: usize,
}
