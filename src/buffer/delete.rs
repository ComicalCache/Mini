use crate::{
    cursor::{self, Cursor},
    document::Document,
    history::{Change, History},
    viewport::Viewport,
};

macro_rules! delete_fn {
    ($func:ident, $func_call:ident, $comment:meta $(,$n:ident)?) => {
        #[$comment]
        pub fn $func(
            doc: &mut Document,
            view: &mut Viewport,
            history: Option<&mut History>,
            $($n: usize,)?
        ) {
            let tmp = doc.cur;
            cursor::$func_call(doc, view $(,$n)?);
            selection(doc, view, &mut Some(tmp), history);
        }
    };
}

#[macro_export]
/// Convenience macro for calling deletion functions. Expects a `BaseBuffer` as member `base`.
macro_rules! delete {
    ($self:ident, $func:ident) => {{
        $crate::buffer::delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            Some(&mut $self.history),
        );
        $self.base.clear_matches();
    }};
    ($self:ident, $func:ident, REPEAT) => {{
        $crate::buffer::delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            Some(&mut $self.history),
            1,
        );
        $self.base.clear_matches();
    }};
    ($self:ident, $func:ident, SELECTION) => {{
        $crate::buffer::delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            &mut $self.base.sel,
            Some(&mut $self.history),
        );
        $self.base.clear_matches();
    }};
}

#[macro_export]
/// Convenience macro for calling change functions. Expects a `BaseBuffer` as member `base`.
macro_rules! change {
    ($self:ident, $func:ident) => {{
        $crate::buffer::delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            Some(&mut $self.history),
        );
        $self.base.change_mode(Mode::Other(Write));
    }};
    ($self:ident, $func:ident, REPEAT) => {{
        $crate::buffer::delete::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            Some(&mut $self.history),
            1,
        );
        $self.base.change_mode(Mode::Other(Write));
    }};
}

/// Deletes the selected area.
pub fn selection(
    doc: &mut Document,
    view: &mut Viewport,
    sel: &mut Option<Cursor>,
    history: Option<&mut History>,
) {
    let Some(pos) = *sel else {
        return;
    };

    let cur = doc.cur;
    let (start, end) = if pos <= cur { (pos, cur) } else { (cur, pos) };

    if let Some(history) = history
        && let Some(data) = doc.get_range(start, end)
    {
        history.add_change(Change::Delete {
            pos: start,
            data: data.to_string(),
        });
    }

    doc.remove_range(start, end);

    // Place cursor at the beginning of the deleted area.
    cursor::move_to(doc, view, start);

    *sel = None;
}

/// Deletes a line.
pub fn line(doc: &mut Document, view: &mut Viewport, history: Option<&mut History>, n: usize) {
    if doc.len() == 1 && doc.line(0).unwrap().len_chars() == 0 {
        return;
    }

    if doc.cur.y + n >= doc.len() {
        cursor::up(doc, view, doc.cur.y + n - doc.len());
    }

    // Begin of selection at the end of one line above the first line or at beginning of current line
    // if in the first line.
    let tmp1 = doc.cur;
    cursor::up(doc, view, 1);
    if tmp1.y != 0 {
        cursor::jump_to_end_of_line(doc, view);
    } else {
        cursor::jump_to_beginning_of_line(doc, view);
    }

    // End selection at the end of the last line or at the beginning of the next line if selection started
    // in the first line.
    let tmp2 = doc.cur;
    cursor::down(doc, view, n);
    if tmp1.y != 0 || tmp2.y + 1 == doc.len() {
        cursor::jump_to_end_of_line(doc, view);
    } else {
        cursor::jump_to_beginning_of_line(doc, view);
    }

    selection(doc, view, &mut Some(tmp2), history);

    // Fix cursor moving up due to moving it one line up.
    if tmp1.y != 0 {
        cursor::down(doc, view, 1);
    }
    cursor::jump_to_beginning_of_line(doc, view);
}

delete_fn!(left, left, doc = "Deletes left of the cursor.", n);
delete_fn!(right, right, doc = "Deletes right of the cursor.", n);
delete_fn!(next_word, next_word, doc = "Deletes the next word.", n);
delete_fn!(prev_word, prev_word, doc = "Deletes the previous word.", n);
delete_fn!(
    next_word_end,
    next_word_end,
    doc = "Deletes to the end of the next word.",
    n
);
delete_fn!(
    prev_word_end,
    prev_word_end,
    doc = "Deletes to the end of the previous word.",
    n
);
delete_fn!(
    next_whitespace,
    next_whitespace,
    doc = "Deletes to the next whitespace.",
    n
);
delete_fn!(
    prev_whitespace,
    prev_whitespace,
    doc = "Deletes to the previous whitespace.",
    n
);
delete_fn!(
    next_empty_line,
    next_empty_line,
    doc = "Deletes to the next empty line.",
    n
);
delete_fn!(
    prev_empty_line,
    prev_empty_line,
    doc = "Deletes to the previous empty line.",
    n
);
delete_fn!(
    beginning_of_line,
    jump_to_beginning_of_line,
    doc = "Deletes until the beginning of the line."
);
delete_fn!(
    end_of_line,
    jump_to_end_of_line,
    doc = "Deletes until the end of the line."
);
delete_fn!(
    matching_opposite,
    jump_to_matching_opposite,
    doc = "Deletes until the matching opposite bracket."
);
delete_fn!(
    beginning_of_file,
    jump_to_beginning_of_file,
    doc = "Deletes until the beginning of the file."
);
delete_fn!(
    end_of_file,
    jump_to_end_of_file,
    doc = "Deletes until the end of the file."
);
