use crate::viewport::TXT;
use std::{collections::HashMap, path::Path};
use termion::color::{Fg, Rgb};
use tree_sitter_highlight::{
    HighlightConfiguration as TsHighlightConfiguration, HighlightEvent as TsHighlightEvent,
    Highlighter as TsHighlighter,
};

macro_rules! parser_configs {
    ($input:expr, $names:expr, $(($lang_name:literal, $lang:expr, $name:literal, $highlights:expr,),)+) => {
        match $input {
            $(
            $lang_name => {
                let mut config =
                    TsHighlightConfiguration::new($lang, $name, $highlights, "", "").unwrap();
                config.configure($names);

                Some(config)
            }
            )+
            "off" => None,
            _ => return false,
        }
    };
}

macro_rules! extension_configs {
    ($self:ident, $extension:expr, $(($ext:literal, $lang:literal),)+) => {
        match $extension {
            $(
                $ext => $self.set_lang($lang),
            )+
            _ => $self.set_lang("off"),
        }
    };
}

/// A highlighter that can highlight document contents.
pub struct Highlighter {
    /// Tree sitter highlighter.
    #[allow(clippy::struct_field_names)]
    highlighter: TsHighlighter,
    /// Tree sitter highlighter config.
    config: Option<TsHighlightConfiguration>,

    /// Events emitted during highlighting.
    pub highlights: Vec<TsHighlightEvent>,

    /// Names of captures to highlight.
    pub names: [&'static str; 36],
    /// Mapping of capture names to color.
    pub colors: HashMap<&'static str, Fg<Rgb>>,
}

impl Highlighter {
    pub fn new() -> Self {
        // FIXME: make this configurable?
        let names = [
            "function",
            "function.builtin",
            "function.call",
            "function.macro",
            "keyword",
            "keyword.function",
            "keyword.operator",
            "keyword.return",
            "operator",
            "storage.type",
            "string",
            "string.escape",
            "string.special",
            "comment",
            "type",
            "type.builtin",
            "type.definition",
            "constructor",
            "variable",
            "variable.builtin",
            "variable.parameter",
            "constant",
            "constant.builtin",
            "number",
            "boolean",
            "punctuation.bracket",
            "punctuation.delimiter",
            "punctuation.special",
            "attribute",
            "property",
            "namespace",
            "module",
            "label",
            "include",
            "tag",
            "field",
        ];
        // FIXME: make this configurable?
        let colors = HashMap::from([
            // Atom Blue: #61afef -> Rgb(97, 175, 239)
            ("function", Fg(Rgb(97, 175, 239))),
            ("function.builtin", Fg(Rgb(97, 175, 239))),
            ("function.call", Fg(Rgb(97, 175, 239))),
            ("function.macro", Fg(Rgb(97, 175, 239))),
            ("constructor", Fg(Rgb(97, 175, 239))),
            ("module", Fg(Rgb(97, 175, 239))),
            // Atom Magenta: #c678dd -> Rgb(198, 120, 221)
            ("keyword", Fg(Rgb(198, 120, 221))),
            ("keyword.function", Fg(Rgb(198, 120, 221))),
            ("keyword.return", Fg(Rgb(198, 120, 221))),
            ("storage.type", Fg(Rgb(198, 120, 221))),
            ("label", Fg(Rgb(198, 120, 221))),
            ("include", Fg(Rgb(198, 120, 221))),
            // Atom Cyan: #56b6c2 -> Rgb(86, 182, 194)
            ("keyword.operator", Fg(Rgb(86, 182, 194))),
            ("operator", Fg(Rgb(86, 182, 194))),
            ("string.escape", Fg(Rgb(86, 182, 194))),
            ("punctuation.special", Fg(Rgb(86, 182, 194))),
            // Atom Green: #98c379 -> Rgb(152, 195, 121)
            ("string", Fg(Rgb(152, 195, 121))),
            ("string.special", Fg(Rgb(152, 195, 121))),
            // Atom Grey: #5c6370 -> Rgb(92, 99, 112)
            ("comment", Fg(Rgb(92, 99, 112))),
            // Atom Yellow: #e5c07b -> Rgb(229, 192, 123)
            ("type", Fg(Rgb(229, 192, 123))),
            ("type.definition", Fg(Rgb(229, 192, 123))),
            ("namespace", Fg(Rgb(229, 192, 123))),
            // Atom Orange: #d19a66 -> Rgb(209, 154, 102)
            ("type.builtin", Fg(Rgb(209, 154, 102))),
            ("constant", Fg(Rgb(209, 154, 102))),
            ("constant.builtin", Fg(Rgb(209, 154, 102))),
            ("number", Fg(Rgb(209, 154, 102))),
            ("boolean", Fg(Rgb(209, 154, 102))),
            ("attribute", Fg(Rgb(209, 154, 102))),
            // Atom Red: #e06c75 -> Rgb(224, 108, 117)
            ("variable.builtin", Fg(Rgb(224, 108, 117))),
            ("tag", Fg(Rgb(224, 108, 117))),
            ("variable", Fg(Rgb(224, 108, 117))),
            ("variable.parameter", Fg(Rgb(224, 108, 117))),
            ("property", Fg(Rgb(224, 108, 117))),
            ("field", Fg(Rgb(224, 108, 117))),
            // Atom Light Grey (Default Text): #abb2bf -> Rgb(171, 178, 191)
            ("punctuation.bracket", TXT),
            ("punctuation.delimiter", TXT),
        ]);

        Self {
            highlighter: TsHighlighter::new(),
            config: None,
            highlights: Vec::new(),
            names,
            colors,
        }
    }

    /// Computes the syntax highlighting for a given `String`.
    pub fn highlight(&mut self, contents: &String) {
        let Some(config) = self.config.as_ref() else {
            // Set the source to be the entire contents if no language for highlighting has been configured.
            self.highlights.clear();
            self.highlights.push(TsHighlightEvent::Source {
                start: 0,
                end: contents.len(),
            });
            return;
        };

        let highlights = self
            .highlighter
            .highlight(config, contents.as_bytes(), None, |_| None)
            .unwrap();
        self.highlights.clear();
        self.highlights
            .clone_from(&highlights.map(Result::unwrap).collect());
    }

    /// Configures the language to do the syntax highlighting for.
    pub fn set_lang(&mut self, lang: &str) -> bool {
        let config = parser_configs!(
            lang,
            &self.names,
            (
                "rust",
                tree_sitter_rust::LANGUAGE.into(),
                "Rust",
                tree_sitter_rust::HIGHLIGHTS_QUERY,
            ),
            (
                "toml",
                tree_sitter_toml::LANGUAGE.into(),
                "Toml",
                tree_sitter_toml::HIGHLIGHT_QUERY,
            ),
            (
                "python",
                tree_sitter_python::LANGUAGE.into(),
                "Python",
                tree_sitter_python::HIGHLIGHTS_QUERY,
            ),
            (
                "ts",
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                "TypeScript",
                tree_sitter_typescript::HIGHLIGHTS_QUERY,
            ),
            (
                "tsx",
                tree_sitter_typescript::LANGUAGE_TSX.into(),
                "TSX",
                tree_sitter_typescript::HIGHLIGHTS_QUERY,
            ),
            (
                "cpp",
                tree_sitter_cpp::LANGUAGE.into(),
                "C++",
                tree_sitter_cpp::HIGHLIGHT_QUERY,
            ),
            (
                "js",
                tree_sitter_javascript::LANGUAGE.into(),
                "JavaScript",
                tree_sitter_javascript::HIGHLIGHT_QUERY,
            ),
            (
                "go",
                tree_sitter_go::LANGUAGE.into(),
                "Go",
                tree_sitter_go::HIGHLIGHTS_QUERY,
            ),
            (
                "c",
                tree_sitter_c::LANGUAGE.into(),
                "C",
                tree_sitter_c::HIGHLIGHT_QUERY,
            ),
            (
                "java",
                tree_sitter_java::LANGUAGE.into(),
                "Java",
                tree_sitter_java::HIGHLIGHTS_QUERY,
            ),
            (
                "html",
                tree_sitter_html::LANGUAGE.into(),
                "HTML",
                tree_sitter_html::HIGHLIGHTS_QUERY,
            ),
            (
                "json",
                tree_sitter_json::LANGUAGE.into(),
                "JSON",
                tree_sitter_json::HIGHLIGHTS_QUERY,
            ),
            (
                "css",
                tree_sitter_css::LANGUAGE.into(),
                "CSS",
                tree_sitter_css::HIGHLIGHTS_QUERY,
            ),
            (
                "lua",
                tree_sitter_lua::LANGUAGE.into(),
                "Lua",
                tree_sitter_lua::HIGHLIGHTS_QUERY,
            ),
            (
                "yaml",
                tree_sitter_yaml::LANGUAGE.into(),
                "YAML",
                tree_sitter_yaml::HIGHLIGHTS_QUERY,
            ),
            (
                "zig",
                tree_sitter_zig::LANGUAGE.into(),
                "Zig",
                tree_sitter_zig::HIGHLIGHTS_QUERY,
            ),
        );

        self.config = config;
        true
    }

    /// Configures the language based on the file extension.
    pub fn set_lang_filename<P: AsRef<Path>>(&mut self, path: P) {
        let Some(extension) = path.as_ref().extension().map(|e| e.to_string_lossy()) else {
            return;
        };

        extension_configs!(
            self,
            extension.as_ref(),
            ("rs", "rust"),
            ("toml", "toml"),
            ("py", "python"),
            ("ts", "ts"),
            ("tsx", "tsx"),
            ("cpp", "cpp"),
            // FIXME: this might cause problems for C header files.
            ("h", "cpp"),
            ("js", "js"),
            ("go", "go"),
            ("c", "c"),
            ("java", "java"),
            ("html", "html"),
            ("json", "json"),
            ("css", "css"),
            ("lua", "lua"),
            ("yaml", "yaml"),
            ("zig", "zig"),
        );
    }
}
