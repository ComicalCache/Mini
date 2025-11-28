mod apply_command;

use crate::{
    cursor::{self, Cursor},
    document::Document,
    message::{Message, MessageKind},
    viewport::Viewport,
};
use arboard::Clipboard;
use std::io::Error;

/// A base set of buffer mode.
pub enum Mode<T> {
    /// View mode, made for inspecting a document.
    View,
    /// Command mode, made to issue commands to the buffer.
    Command,
    /// Other modes defined by specialized buffers.
    Other(T),
}

/// A struct defining the base functionality of a buffer. Specialized buffers can keep
/// it as a field to "inherit" this base. Buffers with completely separate functionality
/// can use it as a blueprint and define their own functionality from scratch.
pub struct BaseBuffer<ModeEnum> {
    /// Total width of the `Buffer`.
    pub w: usize,
    /// Total height of the `Buffer`.
    pub h: usize,
    /// Total x-axis offset of the `Buffer`.
    pub x_off: usize,
    /// Total y-axis offset of the `Buffer`.
    pub y_off: usize,

    /// The main content of the buffer.
    pub doc: Document,
    /// The command content.
    pub cmd: Document,

    /// The viewport of the document.
    pub doc_view: Viewport,
    /// The viewport of the info bar.
    pub info_view: Viewport,
    /// The viewport of the command line.
    pub cmd_view: Viewport,

    /// Marker of the start of the selection.
    pub sel: Option<Cursor>,
    /// The current buffer mode.
    pub mode: Mode<ModeEnum>,
    /// An instance of the system clipboard to yank to.
    pub clipboard: Clipboard,

    /// The vector of matches of a search.
    matches: Vec<(Cursor, Cursor)>,
    /// The index of the current match for navigation.
    matches_idx: Option<usize>,

    /// The history of entered commands.
    pub cmd_history: Vec<String>,
    /// The current index in the command history.
    pub cmd_history_idx: usize,

    /// The active message.
    pub message: Option<Message>,

    /// Flag if the buffer needs re-rendering.
    pub rerender: bool,
}

impl<ModeEnum> BaseBuffer<ModeEnum> {
    pub fn new(
        w: usize,
        h: usize,
        x_off: usize,
        y_off: usize,
        contents: Option<String>,
    ) -> Result<Self, Error> {
        // Set the command view number width manually.
        // FIXME: this limits the bar to always be exactly one in height.
        let cmd_view = Viewport::new(w, 1, 0, 0, x_off, y_off, None);

        let count = contents.as_ref().map_or(1, |buff| buff.len().max(1));
        Ok(Self {
            w,
            h,
            x_off,
            y_off,
            doc: Document::new(0, 0, contents),
            cmd: Document::new(0, 0, None),
            // Shifted by one because of info/command line.
            // FIXME: this limits the bar to always be exactly one in height.
            doc_view: Viewport::new(w, h - 1, 0, 0, x_off, y_off + 1, Some(count)),
            // FIXME: this limits the bar to always be exactly one in height.
            info_view: Viewport::new(w, 1, 0, 0, x_off, y_off, None),
            cmd_view,
            sel: None,
            mode: Mode::View,
            clipboard: Clipboard::new().map_err(Error::other)?,
            matches: Vec::new(),
            matches_idx: None,
            cmd_history: Vec::new(),
            cmd_history_idx: 0,
            message: None,
            rerender: true,
        })
    }

    /// Resizes the viewports of the buffer.
    pub fn resize(&mut self, w: usize, h: usize, x_off: usize, y_off: usize) {
        let doc_count = self.doc.len();

        self.w = w;
        self.h = h;
        self.x_off = x_off;
        self.y_off = y_off;

        // Shifted by one because of info/command line.
        // FIXME: this limits the bar to always be exactly one in height.
        self.doc_view
            .resize(w, h - 1, x_off, y_off + 1, Some(doc_count));
        // FIXME: this limits the bar to always be exactly one in height.
        self.info_view.resize(w, 1, x_off, y_off, None);
        // FIXME: this limits the bar to always be exactly one in height.
        self.cmd_view.resize(w, 1, x_off, y_off, None);

        if let Some(message) = &mut self.message {
            message.calculate_lines(w);
        }
    }

    /// Jumps to the next search match if any.
    pub fn next_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        let idx = self.matches_idx.as_mut().unwrap();
        *idx = (*idx + 1) % self.matches.len();

        self.sel = Some(self.matches[*idx].1);
        cursor::move_to(&mut self.doc, &mut self.doc_view, self.matches[*idx].0);
    }

    // Jumps to the previous search match if any.
    pub fn prev_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        let idx = self.matches_idx.as_mut().unwrap();
        if *idx != 0 {
            *idx -= 1;
        } else {
            *idx = self.matches.len() - 1;
        }

        self.sel = Some(self.matches[*idx].1);
        cursor::move_to(&mut self.doc, &mut self.doc_view, self.matches[*idx].0);
    }

    /// Clears the existing matches of the buffer.
    pub fn clear_matches(&mut self) {
        self.matches.clear();
        self.matches_idx = None;
    }

    /// Loads the next command history item.
    pub fn next_command_history(&mut self) {
        if self.cmd_history_idx == self.cmd_history.len() {
            return;
        }

        self.cmd_history_idx += 1;
        if self.cmd_history_idx == self.cmd_history.len() {
            self.cmd.from("");
        } else {
            self.cmd
                .from(self.cmd_history[self.cmd_history_idx].as_str());
        }

        cursor::jump_to_end_of_line(&mut self.cmd, &mut self.cmd_view);
    }

    /// Loads the previous command history item.
    pub fn prev_command_history(&mut self) {
        if self.cmd_history_idx == 0 {
            return;
        }

        self.cmd_history_idx -= 1;
        self.cmd
            .from(self.cmd_history[self.cmd_history_idx].as_str());

        cursor::jump_to_end_of_line(&mut self.cmd, &mut self.cmd_view);
    }

    /// Changes the base buffers mode.
    pub fn change_mode(&mut self, new_mode: Mode<ModeEnum>) {
        match self.mode {
            Mode::Command => {
                // Clear command line so its ready for next entry. Don't save contents here since they are only
                // saved when hitting enter.
                self.cmd.from("");
                self.cmd_view.cur = Cursor::new(0, 0);
            }
            Mode::View => {
                // Since search matches could have been overwritten we discard all matches.
                if self.doc.edited {
                    self.clear_matches();
                }
            }
            Mode::Other(_) => {}
        }

        match new_mode {
            Mode::Command => self.cmd_history_idx = self.cmd_history.len(),
            Mode::View | Mode::Other(_) => {}
        }

        self.mode = new_mode;
    }

    /// Set a message to display to the user.
    pub fn set_message(&mut self, kind: MessageKind, text: String) {
        self.message = Some(Message::new(kind, text, self.doc_view.w));
        self.rerender = true;
    }

    /// Clear the displayed message.
    pub fn clear_message(&mut self) {
        self.message = None;
        self.rerender = true;
    }
}
