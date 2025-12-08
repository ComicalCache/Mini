use crate::cursor::Cursor;

/// A change replacing data.
pub struct Replace {
    pub pos: Cursor,
    pub delete_data: String,
    pub insert_data: String,
}

type Change = Vec<Replace>;

/// A history of changes to a document.
pub struct History {
    /// The undo stack of changes.
    undo: Vec<Change>,
    /// The redo stack of changes.
    redo: Vec<Change>,
}

impl History {
    pub const fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Adds a new change to the history.
    pub fn add_change(&mut self, change: Change) {
        self.undo.push(change);
        self.redo.clear();
    }

    /// Pops the last change for undoing.
    pub fn undo(&mut self) -> Option<Change> {
        self.undo.pop()
    }

    /// Pops the last undone change for redoing.
    pub fn redo(&mut self) -> Option<Change> {
        self.redo.pop()
    }

    /// Pushes a change to the redo stack.
    pub fn push_redo(&mut self, change: Change) {
        self.redo.push(change);
    }

    /// Pushes a change to the undo stack.
    pub fn push_undo(&mut self, change: Change) {
        self.undo.push(change);
    }
}
