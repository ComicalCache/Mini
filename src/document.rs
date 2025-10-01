use crate::cursor::Cursor;
use std::{
    borrow::Cow,
    fs::File,
    io::{BufWriter, Error, Seek, SeekFrom, Write},
};

pub struct Document {
    pub buff: Vec<Cow<'static, str>>,
    pub cur: Cursor,
    pub edited: bool,
}

impl Document {
    pub fn new(x: usize, y: usize, contents: Option<Vec<Cow<'static, str>>>) -> Self {
        let lines = contents
            .filter(|c| !c.is_empty())
            .map_or_else(|| vec![Cow::from("")], |c| c.into_iter().collect());

        Document {
            buff: lines,
            cur: Cursor::new(x, y),
            edited: false,
        }
    }

    /// Clears the document and sets the cursor to a specified position.
    pub fn clear(&mut self, x: usize, y: usize) {
        self.buff.truncate(1);
        self.buff[0].to_mut().clear();
        self.cur = Cursor::new(x, y);
        self.edited = false;
    }

    /// Returns the count of chars in a line.
    pub fn line_count(&self, y: usize) -> usize {
        self.buff[y].chars().count()
    }

    /// Writes the document to a specified file.
    pub fn write_to_file(&mut self, file: &mut File) -> Result<(), Error> {
        if !self.edited {
            return Ok(());
        }

        // Plus one for the newline.
        let size: u64 = self.buff.iter().map(|s| s.len() as u64 + 1).sum();
        file.set_len(size.saturating_sub(1))?;

        file.seek(SeekFrom::Start(0))?;
        let mut writer = BufWriter::new(file);
        for line in &self.buff {
            writeln!(writer, "{line}")?;
        }
        writer.flush()?;

        self.edited = false;
        Ok(())
    }

    /// Replaces the document buffer and sets the cursor to a specified position.
    pub fn set_contents(&mut self, lines: &[Cow<'static, str>], x: usize, y: usize) {
        if !lines.is_empty() {
            self.buff.resize(lines.len(), Cow::from(""));
            for (idx, line) in lines.iter().enumerate() {
                self.buff[idx].clone_from(line);
            }
        }
        self.cur = Cursor::new(x, y);
        self.edited = false;
    }

    /// Inserts a new line at the current cursor y position.
    pub fn insert_line(&mut self, line: Cow<'static, str>) {
        self.insert_line_at(self.cur.y, line);
    }

    /// Inserts a new line at a specified y position.
    pub fn insert_line_at(&mut self, y: usize, line: Cow<'static, str>) {
        self.buff.insert(y, line);
        self.edited = true;
    }

    /// Removes a line at the current cursor y position.
    pub fn remove_line(&mut self) -> Cow<'static, str> {
        self.remove_line_at(self.cur.y)
    }

    /// Removes a line at a specified y position.
    pub fn remove_line_at(&mut self, y: usize) -> Cow<'static, str> {
        self.buff.remove(y)
    }

    /// Writes a char at the current cursor position.
    pub fn write_char(&mut self, ch: char) {
        self.write_char_at(self.cur.x, self.cur.y, ch);
    }

    /// Writes a char at a specified position.
    pub fn write_char_at(&mut self, x: usize, y: usize, ch: char) {
        let idx = self.buff[y]
            .char_indices()
            .nth(x)
            .map_or(self.buff[y].len(), |(idx, _)| idx);
        self.buff[y].to_mut().insert(idx, ch);
        self.edited = true;
    }

    /// Deletes a char at the current cursor position.
    pub fn delete_char(&mut self) {
        self.delete_char_at(self.cur.x, self.cur.y);
    }

    /// Deletes a char at a specified position.
    pub fn delete_char_at(&mut self, x: usize, y: usize) {
        let line = &mut self.buff[y];
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
        self.write_str_at(self.cur.x, self.cur.y, r#str);
    }

    /// Writes a str at a specified position.
    pub fn write_str_at(&mut self, x: usize, y: usize, r#str: &str) {
        let line = &mut self.buff[y];
        let idx = line
            .char_indices()
            .nth(x)
            .map_or(line.len(), |(idx, _)| idx);

        line.to_mut().insert_str(idx, r#str);
        self.edited = true;
    }

    /// Copies a range of text from the document.
    pub fn get_range(&self, pos1: Cursor, pos2: Cursor) -> Cow<'static, str> {
        let (start, end) = if pos1 <= pos2 {
            (pos1, pos2)
        } else {
            (pos2, pos1)
        };

        if start.y == end.y {
            let line = &self.buff[start.y];
            let start_idx = line
                .char_indices()
                .nth(start.x)
                .map_or(line.len(), |(idx, _)| idx);
            let end_idx = line
                .char_indices()
                .nth(end.x)
                .map_or(line.len(), |(idx, _)| idx);

            return Cow::from(line[start_idx..end_idx].to_string());
        }

        let mut result = String::new();

        // First line
        let first_line = &self.buff[start.y];
        let start_idx = first_line
            .char_indices()
            .nth(start.x)
            .map_or(first_line.len(), |(idx, _)| idx);
        result.push_str(&first_line[start_idx..]);
        result.push('\n');

        // Lines between first and last line
        for line in self.buff.iter().skip(start.y + 1).take(end.y - start.y - 1) {
            result.push_str(line);
            result.push('\n');
        }

        // Last line
        let last_line = &self.buff[end.y];
        let end_idx = last_line
            .char_indices()
            .nth(end.x)
            .map_or(last_line.len(), |(idx, _)| idx);
        result.push_str(&last_line[..end_idx]);

        Cow::from(result)
    }
}
