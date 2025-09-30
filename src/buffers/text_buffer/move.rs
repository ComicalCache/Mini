use crate::buffers::text_buffer::TextBuffer;

impl TextBuffer {
    /// Moves the command cursor to the left.
    pub(super) fn cmd_left(&mut self, n: usize) {
        self.cmd.cur.left(n);
        self.view.cur.left(n);
    }

    /// Moves the command cursor to the right.
    pub(super) fn cmd_right(&mut self, n: usize) {
        let line_bound = self.cmd.buff[0].chars().count();
        self.cmd.cur.right(n, line_bound);
        self.view.cur.right(n, line_bound.min(self.view.w - 1));
    }
}
