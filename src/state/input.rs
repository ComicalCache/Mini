use crate::{
    state::State,
    util::{CursorMove, Position},
};

const TAB: &str = "    ";

impl State {
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

    /// Inserts a new line above the current line and moves to it
    pub fn insert_move_new_line_above(&mut self) {
        self.line_buff.insert(self.txt_pos.y, String::new());
        // No need to move since the cursor pos stays the same
        self.edited = true;
    }

    /// Inserts a new line bellow the current line and moves to it
    pub fn insert_move_new_line_bellow(&mut self) {
        self.line_buff.insert(self.txt_pos.y + 1, String::new());
        self.move_cursor(CursorMove::Down, 1);
        self.edited = true;
    }

    /// Writes a character to the buffer
    pub fn write_char(&mut self, ch: char) {
        let idx = self.line_buff[self.txt_pos.y]
            .char_indices()
            .nth(self.txt_pos.x)
            .map_or(self.line_buff[self.txt_pos.y].len(), |(idx, _)| idx);
        self.line_buff[self.txt_pos.y].insert(idx, ch);

        self.txt_pos.x += 1;
        self.term_content_pos.x = (self.term_content_pos.x + 1).min(self.screen_dims.w);
        self.edited = true;
    }

    /// Writes a character to the command buffer
    pub fn write_cmd_char(&mut self, ch: char) {
        let idx = self
            .cmd_buff
            .char_indices()
            .nth(self.cmd_pos.x)
            .map_or(self.cmd_buff.len(), |(idx, _)| idx);
        self.cmd_buff.insert(idx, ch);

        self.cmd_pos.x += 1;
        self.term_cmd_pos.x = (self.cmd_pos.x + 1).min(self.screen_dims.w);
    }

    /// Writes a new line to the buffer, splitting an existing line if necessary
    pub fn write_new_line(&mut self) {
        let line = &mut self.line_buff[self.txt_pos.y];
        let idx = line
            .char_indices()
            .nth(self.txt_pos.x)
            .map_or(line.len(), |(idx, _)| idx);

        let new_line = line.split_off(idx);
        self.line_buff.insert(self.txt_pos.y + 1, new_line);

        self.move_cursor(CursorMove::Down, 1);
        self.move_cursor(CursorMove::Left, self.txt_pos.x);
        self.edited = true;
    }

    /// Writes a tab character to the buffer
    pub fn write_tab(&mut self) {
        let idx = self.line_buff[self.txt_pos.y]
            .char_indices()
            .nth(self.txt_pos.x)
            .map_or(self.line_buff[self.txt_pos.y].len(), |(idx, _)| idx);
        self.line_buff[self.txt_pos.y].insert_str(idx, TAB);

        self.move_cursor(CursorMove::Right, TAB.chars().count());
        self.edited = true;
    }

    /// Writes a tab character to the command buffer
    pub fn write_cmd_tab(&mut self) {
        let idx = self
            .cmd_buff
            .char_indices()
            .nth(self.cmd_pos.x)
            .map_or(self.cmd_buff.len(), |(idx, _)| idx);
        self.cmd_buff.insert_str(idx, TAB);

        self.move_cmd_cursor(CursorMove::Right, TAB.chars().count());
    }

    /// Deletes a character from the buffer, joining two lines if necessary
    pub fn delete_char(&mut self) {
        if self.txt_pos.x > 0 {
            // If deleting a character in a line
            let line = &mut self.line_buff[self.txt_pos.y];
            let idx = line
                .char_indices()
                .nth(self.txt_pos.x - 1)
                .map(|(idx, _)| idx)
                // Safe to unwrap
                .unwrap();

            line.remove(idx);
            self.move_cursor(CursorMove::Left, 1);
            self.edited = true;
        } else if self.txt_pos.y > 0 {
            // If deleting at the beginning of a line (don't delete the first line)
            let prev_line_len = self.line_buff[self.txt_pos.y - 1].chars().count();
            let line = self.line_buff.remove(self.txt_pos.y);
            self.line_buff[self.txt_pos.y - 1].push_str(&line);

            self.move_cursor(CursorMove::Up, 1);
            self.move_cursor(CursorMove::Right, prev_line_len);
            self.edited = true;
        }
    }

    /// Deletes a character from the command buffer
    pub fn delete_cmd_char(&mut self) {
        if self.cmd_pos.x > 0 {
            let idx = self
                .cmd_buff
                .char_indices()
                .nth(self.cmd_pos.x - 1)
                .map(|(idx, _)| idx)
                // Safe to unwrap
                .unwrap();

            self.cmd_buff.remove(idx);
            self.move_cmd_cursor(CursorMove::Left, 1);
        }
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
                        return Some(Position { x, y });
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
                        return Some(Position { x, y });
                    }
                }
            }
        }

        None
    }

    /// Jumps to the matching opposite bracket if on a bracket
    pub fn jump_to_matching_opposite(&mut self) {
        let Some(Position { x, y }) = self.find_matching_bracket() else {
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
}
