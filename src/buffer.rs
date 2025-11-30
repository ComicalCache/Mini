pub mod base;
pub mod delete;
pub mod edit;
pub mod yank;

use crate::{
    display::Display,
    message::{Message, MessageKind},
};
use termion::event::Key;

/// The result of a command entered by the user.
pub enum BufferResult {
    Ok,
    Change(usize),
    Info(String),
    Error(String),
    ListBuffers,
    NewBuffer(BufferKind),
    Init(Box<dyn Buffer>),
    Log,
    Quit,
    ForceQuit,
}

/// Enum of all available `Buffer` kinds.
#[derive(Clone, Copy)]
pub enum BufferKind {
    Text,
    Files,
}

impl BufferKind {
    /// Converts a `String` to a kind.
    pub fn from(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "files" | "f" => Some(Self::Files),
            "text" | "t" => Some(Self::Text),
            _ => None,
        }
    }

    /// Lists all available kinds.
    pub fn list() -> String {
        "Text\nList".to_string()
    }
}

/// The buffer trait defines the basic primitives a buffer needs.
pub trait Buffer {
    /// Returns the kind of the buffer.
    fn kind(&self) -> BufferKind;

    /// Returns the "name" of a buffer.
    fn name(&self) -> String;

    /// Checks if the buffer needs to be rerendered.
    fn need_rerender(&self) -> bool;

    /// Renders the buffer to a `Display`.
    fn render(&mut self, display: &mut Display);

    /// Handles the event, that the terminal was resized.
    fn resize(&mut self, w: usize, h: usize, x_off: usize, y_off: usize);

    /// Processes one tick. A tick is either:
    /// - an immediate tick on input with the corresponding key
    /// - a periodic empty tick on no input
    ///
    /// Thus it should not be assuemed that a tick is always of periodic nature.
    fn tick(&mut self, key: Option<Key>) -> BufferResult;

    /// Gets the buffer's message.
    fn get_message(&self) -> Option<Message>;

    /// Set the buffer's message.
    fn set_message(&mut self, kind: MessageKind, text: String);

    /// Asks if the buffer is ready to quit/has pending changes.
    fn can_quit(&self) -> Result<(), String>;
}
