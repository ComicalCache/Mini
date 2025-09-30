use crate::cursor::Cursor;
use std::{
    borrow::Cow,
    fs::File,
    io::{BufWriter, Error, Seek, SeekFrom, Write},
};

pub struct Document {
    pub lines: Vec<Cow<'static, str>>,
    pub cursor: Cursor,
    pub edited: bool,
}

impl Document {
    pub fn new(contents: Option<Vec<Cow<'static, str>>>, x: usize, y: usize) -> Self {
        let lines = contents
            .filter(|c| !c.is_empty())
            .map_or_else(|| vec![Cow::from("")], |c| c.into_iter().collect());

        Document {
            lines,
            cursor: Cursor::new(x, y),
            edited: false,
        }
    }

    /// Clears the document and sets the cursor to a specified position.
    pub fn clear(&mut self, x: usize, y: usize) {
        self.lines.truncate(1);
        self.lines[0] = Cow::from("");
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
    pub fn set_contents(&mut self, lines: &[Cow<'static, str>], x: usize, y: usize) {
        if !lines.is_empty() {
            self.lines.resize(lines.len(), Cow::from(""));
            for (idx, line) in lines.iter().enumerate() {
                self.lines[idx].clone_from(line);
            }
        }
        self.cursor = Cursor::new(x, y);
        self.edited = false;
    }

    /// Inserts a new line at the current cursor y position.
    pub fn insert_line(&mut self, line: Cow<'static, str>) {
        self.insert_line_at(self.cursor.y, line);
    }

    /// Inserts a new line at a specified y position.
    pub fn insert_line_at(&mut self, y: usize, line: Cow<'static, str>) {
        self.lines.insert(y, line);
        self.edited = true;
    }

    /// Removes a line at the current cursor y position.
    pub fn remove_line(&mut self) -> Cow<'static, str> {
        self.remove_line_at(self.cursor.y)
    }

    /// Removes a line at a specified y position.
    pub fn remove_line_at(&mut self, y: usize) -> Cow<'static, str> {
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
        self.lines[y].to_mut().insert(idx, ch);
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

        line.to_mut().remove(idx);
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

        line.to_mut().insert_str(idx, r#str);
        self.edited = true;
    }
}
