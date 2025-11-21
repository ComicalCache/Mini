use crate::document::highlight_event::HighlightEvent;
use ropey::iter::Chunks;
use std::path::Path;

/// A highlighter that can highlight document contents.
pub struct Highlighter {
    /// Events emitted during highlighting.
    pub highlights: Vec<HighlightEvent>,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            highlights: Vec::new(),
        }
    }

    /// Computes the syntax highlighting for a given `String`.
    pub fn highlight(&mut self, contents: Chunks) {
        // Set the source to be the entire contents if no language for highlighting has been configured.
        self.highlights.clear();
        self.highlights.push(HighlightEvent::Source {
            start: 0,
            end: contents.map(str::len).sum(),
        });
    }

    /// Configures the language to do the syntax highlighting for.
    pub fn set_lang(&mut self, lang: &str) -> bool {
        false
    }

    /// Configures the language based on the file extension.
    pub fn set_lang_filename<P: AsRef<Path>>(&mut self, path: P) {}
}
