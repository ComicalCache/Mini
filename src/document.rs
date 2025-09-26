use crate::cursor::Cursor;

pub struct Document {
    pub lines: Vec<String>,
    pub cursor: Cursor,
    pub edited: bool,
}

impl Document {
    pub fn new(content: Option<Vec<String>>) -> Self {
        let mut doc = Document {
            lines: vec![String::new()],
            cursor: Cursor::new(0, 0),
            edited: false,
        };

        let Some(content) = content else {
            return doc;
        };
        if !content.is_empty() {
            doc.lines.resize(content.len(), String::new());
            for (idx, line) in content.iter().enumerate() {
                doc.lines[idx].replace_range(.., line);
            }
        }

        doc
    }
}
