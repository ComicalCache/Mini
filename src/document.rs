use crate::cursor::Cursor;
use std::{
    borrow::Cow,
    fs::File,
    io::{BufWriter, Error, Seek, SeekFrom, Write},
};

// The document of a buffer containing its contents.
pub struct Document {
    // The buffer contents.
    pub buff: Vec<Cow<'static, str>>,
    // The cursor inside the buffer.
    pub cur: Cursor,
    // Flag if the buffer was modified.
    pub edited: bool,
}

impl Document {
    pub fn new(x: usize, y: usize, contents: Option<Vec<Cow<'static, str>>>) -> Self {
        // Always have at least one line in the document.
        let buff = contents
            .filter(|c| !c.is_empty())
            .map_or_else(|| vec![Cow::from("")], |c| c.into_iter().collect());

        Document {
            buff,
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
    pub fn line_count(&self, y: usize) -> Option<usize> {
        self.buff.get(y).map(|l| l.chars().count())
    }

    /// Writes the document to a specified file.
    pub fn write_to_file(&mut self, file: &mut File) -> Result<(), Error> {
        if !self.edited {
            return Ok(());
        }

        // Plus one for the newline.
        let size: u64 = self.buff.iter().map(|s| s.len() as u64 + 1).sum();
        file.set_len(size.saturating_sub(1))?;

        // Write line by line.
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
    pub fn set_contents(&mut self, buff: &[Cow<'static, str>], x: usize, y: usize) {
        if buff.is_empty() {
            // Always have at least one line in the document.
            self.buff.truncate(1);
            self.buff[0].to_mut().clear();
        } else {
            self.buff.resize(buff.len(), Cow::from(""));
            for (idx, line) in buff.iter().enumerate() {
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
        if y > self.buff.len() {
            return;
        }

        self.buff.insert(y, line);
        self.edited = true;
    }

    /// Removes a line at the current cursor y position.
    pub fn remove_line(&mut self) -> Option<Cow<'static, str>> {
        self.remove_line_at(self.cur.y)
    }

    /// Removes a line at a specified y position.
    pub fn remove_line_at(&mut self, y: usize) -> Option<Cow<'static, str>> {
        if y >= self.buff.len() {
            return None;
        }

        let line = self.buff.remove(y);
        if self.buff.is_empty() {
            self.buff.push(Cow::from(""));
        }

        Some(line)
    }

    /// Writes a char at the current cursor position.
    pub fn write_char(&mut self, ch: char) {
        self.write_char_at(self.cur.x, self.cur.y, ch);
    }

    /// Writes a char at a specified position.
    pub fn write_char_at(&mut self, x: usize, y: usize, ch: char) {
        let Some(count) = self.line_count(y) else {
            return;
        };
        if x > count {
            return;
        }

        let idx = self.buff[y]
            .char_indices()
            .nth(x)
            .map_or(self.buff[y].len(), |(idx, _)| idx);

        self.buff[y].to_mut().insert(idx, ch);
        self.edited = true;
    }

    /// Deletes a char at the current cursor position.
    pub fn delete_char(&mut self) -> Option<char> {
        self.delete_char_at(self.cur.x, self.cur.y)
    }

    /// Deletes a char at a specified position.
    pub fn delete_char_at(&mut self, x: usize, y: usize) -> Option<char> {
        if y >= self.buff.len() {
            return None;
        }

        let line = &mut self.buff[y];
        let (idx, _) = line.char_indices().nth(x)?;

        let ret = line.to_mut().remove(idx);
        self.edited = true;

        Some(ret)
    }

    /// Writes a str at the current cursor position.
    /// Creates new lines if the content contains new lines.
    pub fn write_str(&mut self, r#str: &str) {
        self.write_str_at(self.cur.x, self.cur.y, r#str);
    }

    /// Writes a str at a specified position.
    /// Creates new lines if the content contains new lines.
    pub fn write_str_at(&mut self, x: usize, mut y: usize, r#str: &str) {
        let Some(count) = self.line_count(y) else {
            return;
        };
        if x > count {
            return;
        }

        let mut lines = r#str.split('\n');

        // Insertion point of current line.
        let line = &mut self.buff[y];
        let idx = line
            .char_indices()
            .nth(x)
            .map_or(line.len(), |(idx, _)| idx);

        // Content of the original line after the cursor.
        let tail = line.to_mut().split_off(idx);
        line.to_mut().push_str(lines.next().unwrap_or(""));

        // Insert lines.
        for new_line in lines {
            y += 1;
            self.insert_line_at(y, Cow::from(new_line.to_string()));
        }

        // Append tail.
        self.buff[y].to_mut().push_str(&tail);

        self.edited = true;
    }

    /// Removes a range of text from the document.
    pub fn remove_range(&mut self, pos1: Cursor, pos2: Cursor) {
        let (start, end) = if pos1 <= pos2 {
            (pos1, pos2)
        } else {
            (pos2, pos1)
        };

        if start.y == end.y {
            // Single-line deletion.
            let line = &mut self.buff[start.y];
            let start_idx = line
                .char_indices()
                .nth(start.x)
                .map_or(line.len(), |(idx, _)| idx);
            let end_idx = line
                .char_indices()
                .nth(end.x)
                .map_or(line.len(), |(idx, _)| idx);

            line.to_mut().drain(start_idx..end_idx);
        } else {
            // Multi-line deletion.
            let end_line = &self.buff[end.y];
            let end_idx = end_line
                .char_indices()
                .nth(end.x)
                .map_or(end_line.len(), |(idx, _)| idx);
            let tail = end_line[end_idx..].to_string();

            let start_line = &mut self.buff[start.y];
            let start_idx = start_line
                .char_indices()
                .nth(start.x)
                .map_or(start_line.len(), |(idx, _)| idx);
            start_line.to_mut().truncate(start_idx);
            start_line.to_mut().push_str(&tail);

            // Remove the in-between lines.
            self.buff.drain(start.y + 1..=end.y);
        }

        self.edited = true;
    }

    /// Copies a range of text from the document.
    pub fn get_range(&self, pos1: Cursor, pos2: Cursor) -> Option<Cow<'static, str>> {
        let (start, end) = if pos1 <= pos2 {
            (pos1, pos2)
        } else {
            (pos2, pos1)
        };

        let start_len = self.line_count(start.y)?;
        let end_len = self.line_count(end.y)?;
        if start_len < start.x || end_len < end.x {
            return None;
        }

        if start.y == end.y {
            let line = &self.buff[start.y];
            let start_idx = line
                .char_indices()
                .nth(start.x)
                .map_or(start_len, |(idx, _)| idx);
            let end_idx = line
                .char_indices()
                .nth(end.x)
                .map_or(end_len, |(idx, _)| idx);

            return Some(Cow::from(line[start_idx..end_idx].to_string()));
        }

        // First line
        let first_line = &self.buff[start.y];
        let start_idx = first_line
            .char_indices()
            .nth(start.x)
            .map_or(start_len, |(idx, _)| idx);
        let mut result = Cow::from(first_line[start_idx..].to_string());
        result.to_mut().push('\n');

        // Lines between first and last line
        for line in self.buff.iter().skip(start.y + 1).take(end.y - start.y - 1) {
            result.to_mut().push_str(line);
            result.to_mut().push('\n');
        }

        // Last line
        let last_line = &self.buff[end.y];
        let end_idx = last_line
            .char_indices()
            .nth(end.x)
            .map_or(end_len, |(idx, _)| idx);
        result.to_mut().push_str(&last_line[..end_idx]);

        Some(result)
    }
}
