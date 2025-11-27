use crate::cursor::Cursor;
use ropey::{
    Rope, RopeSlice,
    iter::{Chunks, Lines},
};
use std::{
    fs::File,
    io::{BufWriter, Error, Seek, SeekFrom, Write},
};

// The document of a buffer containing its contents.
pub struct Document {
    // The buffer contents.
    rope: Rope,
    // The cursor inside the buffer.
    pub cur: Cursor,
    // Flag if the buffer was modified.
    pub edited: bool,
}

impl Document {
    pub fn new(x: usize, y: usize, contents: Option<String>) -> Self {
        Self {
            rope: Rope::from_str(contents.unwrap_or_default().as_str()),
            cur: Cursor::new(x, y),
            edited: false,
        }
    }

    /// Initializes the document with new contents.
    pub fn from(&mut self, buff: &str) {
        self.rope = Rope::from_str(buff);
        self.cur = Cursor::new(0, 0);
        self.edited = false;
    }

    /// Returns the number of lines.
    pub fn len(&self) -> usize {
        self.rope.len_lines()
    }

    /// Returns the count of chars in a line.
    pub fn line_count(&self, y: usize) -> Option<usize> {
        if y >= self.len() {
            return None;
        }

        Some(self.rope.line(y).len_chars())
    }

    /// Returns if the line ends with a newline character.
    pub fn ends_with_newline(&self, y: usize) -> bool {
        if y >= self.len() {
            return false;
        }

        self.rope.line(y).chars().last().is_some_and(|l| l == '\n')
    }

    /// Returns a line of the document.
    pub fn line(&self, y: usize) -> Option<RopeSlice<'_>> {
        self.rope.get_line(y)
    }

    /// Returns an iterator over the lines of the document.
    pub fn lines(&self) -> Lines<'_> {
        self.rope.lines()
    }

    /// Writes the document to a specified file.
    pub fn write_to_file(&mut self, file: &mut File) -> Result<(), Error> {
        if !self.edited {
            return Ok(());
        }

        file.set_len(self.rope.len_bytes() as u64)?;
        let mut file = BufWriter::new(file);
        file.seek(SeekFrom::Start(0))?;
        self.rope.write_to(&mut file)?;
        file.flush()?;

        self.edited = false;
        Ok(())
    }

    /// Inserts a new line at a specified y position.
    pub fn insert_line(&mut self, y: usize) {
        self.rope.insert(self.rope.line_to_char(y), "\n");
        self.edited = true;
    }

    /// Writes a char at a specified position.
    pub fn write_char(&mut self, ch: char, x: usize, y: usize) {
        self.rope.insert_char(self.xy_to_idx(x, y), ch);
        self.edited = true;
    }

    /// Deletes a char at a specified position.
    pub fn delete_char(&mut self, x: usize, y: usize) -> char {
        let idx = self.xy_to_idx(x, y);
        let ch = self.rope.char(idx);

        self.rope.remove(idx..=idx);
        self.edited = true;

        ch
    }

    /// Writes a str at the current cursor position.
    /// Creates new lines if the content contains new lines.
    pub fn write_str(&mut self, str: &str) {
        self.write_str_at(self.cur.x, self.cur.y, str);
    }

    /// Writes a str at a specified position.
    /// Creates new lines if the content contains new lines.
    pub fn write_str_at(&mut self, x: usize, y: usize, str: &str) {
        self.rope.insert(self.xy_to_idx(x, y), str);
        self.edited = true;
    }

    /// Gets a range of text from the document.
    pub fn get_range(&self, pos1: Cursor, pos2: Cursor) -> Option<RopeSlice<'_>> {
        let (start, end) = if pos1 <= pos2 {
            (pos1, pos2)
        } else {
            (pos2, pos1)
        };

        let start_idx = self.xy_to_idx(start.x, start.y);
        let end_idx = self.xy_to_idx(end.x, end.y);

        self.rope.get_slice(start_idx..end_idx)
    }

    /// Gets a range of text from the document using absolute byte indices.
    pub fn get_contents(&self) -> Chunks<'_> {
        self.rope.chunks()
    }

    /// Removes a range of text from the document.
    pub fn remove_range(&mut self, pos1: Cursor, pos2: Cursor) {
        let (start, end) = if pos1 <= pos2 {
            (pos1, pos2)
        } else {
            (pos2, pos1)
        };

        let start_idx = self.xy_to_idx(start.x, start.y);
        let end_idx = self.xy_to_idx(end.x, end.y);
        self.rope.remove(start_idx..end_idx);
        self.edited = true;
    }

    /// Converts (x, y) coordinates to a rope index.
    fn xy_to_idx(&self, x: usize, y: usize) -> usize {
        self.rope.line_to_char(y) + x
    }
}
