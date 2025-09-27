use crate::buffers::text_buffer::TextBuffer;

impl TextBuffer {
    /// Moves the cursor to the left.
    pub(super) fn left(&mut self, n: usize) {
        self.doc.cursor.left(n);
        self.view.cursor.left(n);
    }

    pub(super) fn cmd_left(&mut self, n: usize) {
        self.cmd.cursor.left(n);
        self.view.cursor.left(n);
    }

    /// Moves the cursor to the right
    pub(super) fn right(&mut self, n: usize) {
        let line_bound = self.doc.lines[self.doc.cursor.y].chars().count();
        self.doc.cursor.right(n, line_bound);
        self.view.cursor.right(n, line_bound.min(self.view.w - 1));
    }

    pub(super) fn cmd_right(&mut self, n: usize) {
        let line_bound = self.cmd.lines[0].chars().count();
        self.cmd.cursor.right(n, line_bound);
        self.view.cursor.right(n, line_bound.min(self.view.w - 1));
    }

    /// Moves the cursor up.
    pub(super) fn up(&mut self, n: usize) {
        self.doc.cursor.up(n);

        // When moving up, handle case that new line contains less text than previous.
        let line_bound = self.doc.lines[self.doc.cursor.y].chars().count();
        if self.doc.cursor.x >= line_bound {
            let diff = self.doc.cursor.x - line_bound;
            self.doc.cursor.left(diff);
            self.view.cursor.left(diff);
        }
    }

    /// Moves the cursor down.
    pub(super) fn down(&mut self, n: usize) {
        let bound = self.doc.lines.len().saturating_sub(1);
        self.doc.cursor.down(n, bound);

        // When moving down, handle case that new line contains less text than previous.
        let line_bound = self.doc.lines[self.doc.cursor.y].chars().count();
        if self.doc.cursor.x >= line_bound {
            let diff = self.doc.cursor.x - line_bound;
            self.doc.cursor.left(diff);
            self.view.cursor.left(diff);
        }
    }

    /// Jumps to the next "word".
    pub(super) fn next_word(&mut self) {
        let cursor = self.doc.cursor;
        let line = &self.doc.lines[cursor.y];
        // Return early if at end of line.
        if line.chars().count() <= cursor.x + 1 {
            return;
        }

        let Some(curr) = line.chars().nth(cursor.x) else {
            unreachable!("Character must exist under cursor");
        };

        // Find next not alphanumeric character or alphanumeric character if the current character is not.
        let Some((idx, ch)) = line.chars().skip(cursor.x + 1).enumerate().find(|(_, ch)| {
            !ch.is_alphanumeric() || (!curr.is_alphanumeric() && ch.is_alphanumeric())
        }) else {
            // Return early if no next "word" candidate exists.
            return;
        };

        if ch.is_whitespace() {
            // Find next non-whitespace after whitespace.
            let Some((jdx, _)) = line
                .chars()
                .skip(self.view.cursor.x + 1 + idx)
                .enumerate()
                .find(|(_, ch)| !ch.is_whitespace())
            else {
                // Return early if after the whitespace there are no alphanumeric characters.
                return;
            };

            // Move the cursor to the next "word",
            self.right(idx + jdx + 1);
        } else {
            // If it is not whitespace set cursor to the position of the character.
            self.right(idx + 1);
        }
    }

    /// Jumps to the previous "word".
    pub(super) fn prev_word(&mut self) {
        let cursor = self.doc.cursor;

        // Return early if already at beginning of line.
        if cursor.x == 0 {
            return;
        }

        let line = &self.doc.lines[cursor.y];

        // Find next non-whitespace character.
        if let Some((idx, ch)) = line
            .chars()
            .rev()
            .skip(line.chars().count() - cursor.x)
            .enumerate()
            .find(|&(_, ch)| !ch.is_whitespace())
        {
            let mut offset = idx + 1;

            if ch.is_alphanumeric() {
                // If it's alphanumeric, find first character of that sequence of alphanumeric characters.
                offset += line
                    .chars()
                    .rev()
                    .skip(line.chars().count() - cursor.x)
                    .skip(idx + 1)
                    .take_while(|&ch| ch.is_alphanumeric())
                    .count();
            }

            self.left(offset);
        } else {
            // Move to the beginning of line.
            self.left(cursor.x);
        }
    }

    /// Jumps the the beginning of a line.
    pub(super) fn jump_to_beginning_of_line(&mut self) {
        self.left(self.doc.lines[self.doc.cursor.y].chars().count());
    }

    /// Jumps to the end of a line.
    pub(super) fn jump_to_end_of_line(&mut self) {
        self.right(
            self.doc.lines[self.doc.cursor.y]
                .chars()
                .count()
                .saturating_sub(self.doc.cursor.x + 1),
        );
    }

    fn find_matching_bracket(&self) -> Option<(usize, usize)> {
        let cursor = self.doc.cursor;
        let Some(current_char) = self.doc.lines[cursor.y].chars().nth(cursor.x) else {
            return None; // Cursor is at the end of line.
        };

        let (opening, closing, forward) = match current_char {
            '(' => ('(', ')', true),
            '[' => ('[', ']', true),
            '{' => ('{', '}', true),
            '<' => ('<', '>', true),
            ')' => ('(', ')', false),
            ']' => ('[', ']', false),
            '}' => ('{', '}', false),
            '>' => ('<', '>', false),
            _ => return None,
        };

        let mut depth = 1;
        if forward {
            // Search forward from the character after the cursor.
            for y in cursor.y..self.doc.lines.len() {
                let line = &self.doc.lines[y];
                let offset = if y == cursor.y { cursor.x + 1 } else { 0 };

                for (x, ch) in line.char_indices().skip(offset) {
                    if ch == opening {
                        depth += 1;
                    } else if ch == closing {
                        depth -= 1;
                    }

                    if depth == 0 {
                        return Some((y, x));
                    }
                }
            }
        } else {
            // Search backward from the character before the cursor.
            for y in (0..=cursor.y).rev() {
                let line = &self.doc.lines[y];
                let offset = if y == cursor.y {
                    line.chars().count() - cursor.x
                } else {
                    0
                };

                for (x, ch) in line.char_indices().rev().skip(offset) {
                    if ch == closing {
                        depth += 1;
                    } else if ch == opening {
                        depth -= 1;
                    }

                    if depth == 0 {
                        return Some((y, x));
                    }
                }
            }
        }

        None
    }

    /// Jumps to the matching opposite bracket (if exists).
    pub(super) fn jump_to_matching_opposite(&mut self) {
        let cursor = self.doc.cursor;
        let Some((y, x)) = self.find_matching_bracket() else {
            return;
        };

        if y < cursor.y {
            self.up(cursor.y - y);
        } else if y > cursor.y {
            self.down(y - cursor.y);
        }

        if x < cursor.x {
            self.left(cursor.x - x);
        } else if x > cursor.x {
            self.right(x - cursor.x);
        }
    }

    /// Jumps to the last line of the file.
    pub(super) fn jump_to_end_of_file(&mut self) {
        self.down(self.doc.lines.len() - (self.doc.cursor.y + 1));
        self.left(self.doc.cursor.x);
    }

    /// Jumps to the first line of the file.
    pub(super) fn jump_to_beginning_of_file(&mut self) {
        self.up(self.doc.cursor.y + 1);
        self.left(self.doc.cursor.x);
    }
}
