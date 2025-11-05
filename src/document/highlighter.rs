use std::collections::HashMap;
use termion::color::{Fg, Rgb};
use tree_sitter_highlight::{
    HighlightConfiguration as TsHighlightConfiguration, HighlightEvent,
    Highlighter as TsHighlighter,
};
use tree_sitter_rust;

/// A highlighter that can highlight document contents.
pub struct Highlighter {
    /// Tree sitter highlighter.
    highlighter: TsHighlighter,
    /// Tree sitter highlighter config.
    config: Option<TsHighlightConfiguration>,

    /// Events emitted during highlighting.
    highlights: Vec<HighlightEvent>,

    /// Names of captures to highlight.
    names: [&'static str; 8],
    /// Mapping of capture names to color.
    colors: HashMap<&'static str, Fg<Rgb>>,
}

impl Highlighter {
    pub fn new() -> Self {
        // FIXME: make this configurable?
        let names = [
            "keyword", "function", "string", "comment", "type", "variable", "number", "boolean",
        ];

        // TODO: don't default to rust and make this configurable through a command.
        let mut config = TsHighlightConfiguration::new(
            tree_sitter_rust::LANGUAGE.into(),
            "Rust",
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            "",
            "",
        )
        .unwrap();
        config.configure(&names);

        Self {
            highlighter: TsHighlighter::new(),
            config: Some(config),
            highlights: Vec::new(),
            names,
            colors: HashMap::from([
                ("keyword", Fg(Rgb(0, 255, 255))),    // Cyan
                ("function", Fg(Rgb(100, 149, 237))), // Cornflower Blue
                ("string", Fg(Rgb(85, 200, 85))),     // Green
                ("comment", Fg(Rgb(128, 128, 128))),  // Gray
                ("type", Fg(Rgb(255, 0, 255))),       // Magenta
                ("variable", Fg(Rgb(255, 255, 255))), // White
                ("number", Fg(Rgb(255, 255, 0))),     // Yellow
                ("boolean", Fg(Rgb(255, 80, 80))),    // Red
            ]),
        }
    }

    pub fn highlight(&mut self, contents: &String) -> Result<(), tree_sitter_highlight::Error> {
        let Some(config) = self.config.as_ref() else {
            return Ok(());
        };

        let highlights = self
            .highlighter
            .highlight(config, contents.as_bytes(), None, |_| None)?;
        self.highlights.clear();
        for highlight in highlights {
            self.highlights.push(highlight?);
        }

        Ok(())
    }
}
