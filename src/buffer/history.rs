use crate::cursor::Cursor;
use std::borrow::Cow;

/// A reversible change to a document.
pub enum Change {
    /// Text insertion.
    Insert {
        pos: Cursor,
        data: Cow<'static, str>,
    },
    /// Text deletion.
    Delete {
        pos: Cursor,
        data: Cow<'static, str>,
    },
    Replace(Vec<Replace>),
}

/// A change replacing data.
pub struct Replace {
    pub pos: Cursor,
    pub delete_data: Cow<'static, str>,
    pub insert_data: Cow<'static, str>,
}

/// A history of changes to a document.
pub struct History {
    /// The undo stack of changes.
    undo: Vec<Change>,
    /// The redo stack of changes.
    redo: Vec<Change>,
}

impl History {
    pub fn new() -> Self {
        History {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Adds a new change to the history.
    pub fn add_change(&mut self, change: Change) {
        self.undo.push(change);
        self.redo.clear();
    }

    /// Remove a change from the history.
    pub fn pop_change(&mut self) {
        self.undo.pop();
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
