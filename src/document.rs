use crate::cursor::Cursor;
use std::{
    fs::File,
    io::{BufWriter, Error, Seek, SeekFrom, Write},
};

pub struct Document {
    pub lines: Vec<String>,
    pub cursor: Cursor,
    pub edited: bool,
}

impl Document {
    pub fn new(contents: Option<Vec<String>>, x: usize, y: usize) -> Self {
        let mut doc = Document {
            lines: vec![String::new()],
            cursor: Cursor::new(x, y),
            edited: false,
        };

        if let Some(content) = contents
            && !content.is_empty()
        {
            doc.lines.resize(content.len(), String::new());
            for (idx, line) in content.iter().enumerate() {
                doc.lines[idx].clone_from(line);
            }
        }

        doc
    }

    /// Clears the document and sets the cursor to a specified position.
    pub fn clear(&mut self, x: usize, y: usize) {
        self.lines.truncate(1);
        self.lines[0].clear();
        self.cursor = Cursor::new(x, y);
        self.edited = false;
    }

    /// Writes the document to a specified file.
    pub fn write_to_file(&mut self, file: &mut File) -> Result<(), Error> {
        if !self.edited {
            return Ok(());
        }

        let size: u64 = self.lines.iter().map(|s| s.len() as u64 + 1).sum();
        file.set_len(size.saturating_sub(1))?;

        file.seek(SeekFrom::Start(0))?;
        let mut writer = BufWriter::new(file);
        for line in &self.lines {
            writeln!(writer, "{line}")?;
        }
        writer.flush()?;

        self.edited = false;
        Ok(())
    }

    /// Replaces the document buffer and sets the cursor to a specified position.
    pub fn set_contents(&mut self, lines: &[String], x: usize, y: usize) {
        if !lines.is_empty() {
            self.lines.resize(lines.len(), String::new());
            for (idx, line) in lines.iter().enumerate() {
                self.lines[idx].clone_from(line);
            }
        }
        self.cursor = Cursor::new(x, y);
        self.edited = false;
    }

    /// Inserts a new line at the current cursor y position.
    pub fn insert_line(&mut self, line: String) {
        self.insert_line_at(self.cursor.y, line);
    }

    /// Inserts a new line at a specified y position.
    pub fn insert_line_at(&mut self, y: usize, line: String) {
        self.lines.insert(y, line);
        self.edited = true;
    }

    /// Removes a line at the current cursor y position.
    pub fn remove_line(&mut self) -> String {
        self.remove_line_at(self.cursor.y)
    }

    /// Removes a line at a specified y position.
    pub fn remove_line_at(&mut self, y: usize) -> String {
        self.lines.remove(y)
    }

    /// Writes a char at the current cursor position.
    pub fn write_char(&mut self, ch: char) {
        self.write_char_at(self.cursor.x, self.cursor.y, ch);
    }

    /// Writes a char at a specified position.
    pub fn write_char_at(&mut self, x: usize, y: usize, ch: char) {
        let idx = self.lines[y]
            .char_indices()
            .nth(x)
            .map_or(self.lines[y].len(), |(idx, _)| idx);
        self.lines[y].insert(idx, ch);
        self.edited = true;
    }

    /// Deletes a char at the current cursor position.
    pub fn delete_char(&mut self) {
        self.delete_char_at(self.cursor.x, self.cursor.y);
    }

    /// Deletes a char at a specified position.
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

    /// Writes a str at the current cursor position.
    pub fn write_str(&mut self, r#str: &str) {
        self.write_str_at(self.cursor.x, self.cursor.y, r#str);
    }

    /// Writes a str at a specified position.
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
