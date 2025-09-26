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

    pub fn insert_line(&mut self, line: String) {
        self.insert_line_at(self.cursor.y, line);
    }

    pub fn insert_line_at(&mut self, y: usize, line: String) {
        self.lines.insert(y, line);
        self.edited = true;
    }

    pub fn remove_line(&mut self) -> String {
        self.lines.remove(self.cursor.y)
    }

    pub fn remove_line_at(&mut self, y: usize) -> String {
        self.lines.remove(y)
    }

    pub fn write_char(&mut self, ch: char) {
        self.write_char_at(self.cursor.x, self.cursor.y, ch);
    }

    pub fn write_char_at(&mut self, x: usize, y: usize, ch: char) {
        let idx = self.lines[y]
            .char_indices()
            .nth(x)
            .map_or(self.lines[y].len(), |(idx, _)| idx);
        self.lines[y].insert(idx, ch);
        self.edited = true;
    }

    pub fn delete_char(&mut self) {
        self.delete_char_at(self.cursor.x, self.cursor.y);
    }

    pub fn delete_char_at(&mut self, x: usize, y: usize) {
        let line = &mut self.lines[y];
        let idx = line
            .char_indices()
            .nth(x)
            .map(|(idx, _)| idx)
            // Safe to unwrap
            .unwrap();

        line.remove(idx);
        self.edited = true;
    }

    pub fn write_str(&mut self, r#str: &str) {
        self.write_str_at(self.cursor.x, self.cursor.y, r#str);
    }

    pub fn write_str_at(&mut self, x: usize, y: usize, r#str: &str) {
        let line = &mut self.lines[y];
        let idx = line
            .char_indices()
            .nth(x)
            .map_or(line.len(), |(idx, _)| idx);

        line.insert_str(idx, r#str);
        self.edited = true;
    }
}
