use crate::{
    buffer::Buffer,
    util::{CursorMove, Position},
};

impl Buffer {
    /// Skip to the "next word" in the line
    pub fn next_word(&mut self) {
        let line = &self.line_buff[self.txt_pos.y];
        // Return early if at end of line
        if line.chars().count() <= self.txt_pos.x + 1 {
            return;
        }

        let Some(curr) = line.chars().nth(self.txt_pos.x) else {
            unreachable!("Character must exist under cursor");
        };

        // Find next not alphanumeric character or alphanumeric character if the current character is not
        let Some((idx, ch)) = line
            .chars()
            .skip(self.txt_pos.x + 1)
            .enumerate()
            .find(|(_, ch)| {
                !ch.is_alphanumeric() || (!curr.is_alphanumeric() && ch.is_alphanumeric())
            })
        else {
            // Return early if no "next word" candidate exists
            return;
        };

        if ch.is_whitespace() {
            // Find next non-whitespace after whitespace
            let Some((jdx, _)) = line
                .chars()
                .skip(self.term_content_pos.x + 1 + idx)
                .enumerate()
                .find(|(_, ch)| !ch.is_whitespace())
            else {
                // Return early if after the whitespace there are no alphanumeric characters
                return;
            };

            // Move the cursor to the "next word"
            self.move_cursor(CursorMove::Right, idx + jdx + 2);
        } else {
            // If it is not whitespace set cursor to the position of the character
            self.move_cursor(CursorMove::Right, idx + 1);
        }
    }

    /// Skip to the "prev word" in the line
    pub fn prev_word(&mut self) {
        // Return early if already at start of line
        if self.txt_pos.x == 0 {
            return;
        }

        let line = &self.line_buff[self.txt_pos.y];

        // Find next non-whitespace character
        if let Some((idx, ch)) = line
            .chars()
            .rev()
            .skip(line.chars().count() - self.txt_pos.x)
            .enumerate()
            .find(|&(_, ch)| !ch.is_whitespace())
        {
            let mut offset = idx + 1;

            if ch.is_alphanumeric() {
                // If it's alphanumeric, find first character of that sequence of alphanumeric characters
                offset += line
                    .chars()
                    .rev()
                    .skip(line.chars().count() - self.txt_pos.x)
                    .skip(idx + 1)
                    .take_while(|&ch| ch.is_alphanumeric())
                    .count();
            }

            self.move_cursor(CursorMove::Left, offset);
        } else {
            // Move to the start of line
            self.move_cursor(CursorMove::Left, self.txt_pos.x);
        }
    }

    /// Jumps to the start of a line
    pub fn jump_to_start_of_line(&mut self) {
        self.move_cursor(
            CursorMove::Left,
            self.line_buff[self.txt_pos.y].chars().count(),
        );
    }

    /// Jumps to the last character of a line
    pub fn jump_to_end_of_line(&mut self) {
        self.move_cursor(
            CursorMove::Right,
            self.line_buff[self.txt_pos.y]
                .chars()
                .count()
                .saturating_sub(self.txt_pos.x + 1),
        );
    }

    fn find_matching_bracket(&self) -> Option<Position> {
        let Some(current_char) = self.line_buff[self.txt_pos.y].chars().nth(self.txt_pos.x) else {
            return None; // Cursor is at the end of line
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
            // Search forward from the character after the cursor
            for y in self.txt_pos.y..self.line_buff.len() {
                let line = &self.line_buff[y];
                let offset = if y == self.txt_pos.y {
                    self.txt_pos.x + 1
                } else {
                    0
                };

                for (x, ch) in line.char_indices().skip(offset) {
                    if ch == opening {
                        depth += 1;
                    } else if ch == closing {
                        depth -= 1;
                    }

                    if depth == 0 {
                        return Some(Position::new(y, x));
                    }
                }
            }
        } else {
            // Search backward from the character before the cursor
            for y in (0..=self.txt_pos.y).rev() {
                let line = &self.line_buff[y];
                let offset = if y == self.txt_pos.y {
                    line.chars().count() - self.txt_pos.x
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
                        return Some(Position::new(y, x));
                    }
                }
            }
        }

        None
    }

    /// Jumps to the matching opposite bracket if on a bracket
    pub fn jump_to_matching_opposite(&mut self) {
        let Some(Position { y, x }) = self.find_matching_bracket() else {
            return;
        };

        if y < self.txt_pos.y {
            self.move_cursor(CursorMove::Up, self.txt_pos.y - y);
        } else if y > self.txt_pos.y {
            self.move_cursor(CursorMove::Down, y - self.txt_pos.y);
        }

        if x < self.txt_pos.x {
            self.move_cursor(CursorMove::Left, self.txt_pos.x - x);
        } else if x > self.txt_pos.x {
            self.move_cursor(CursorMove::Right, x - self.txt_pos.x);
        }
    }

    /// Jumps to the end of the buffer
    pub fn jump_to_end(&mut self) {
        self.move_cursor(
            CursorMove::Down,
            self.line_buff.len() - (self.txt_pos.y + 1),
        );
        self.move_cursor(CursorMove::Left, self.txt_pos.x);
    }

    /// Jumps to the start of the buffer
    pub fn jump_to_start(&mut self) {
        self.move_cursor(CursorMove::Up, self.txt_pos.y + 1);
        self.move_cursor(CursorMove::Left, self.txt_pos.x);
    }
}
