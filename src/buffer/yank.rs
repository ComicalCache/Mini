use crate::{
    cursor::{self, Cursor},
    document::Document,
    util::CommandResult,
    viewport::Viewport,
};
use arboard::Clipboard;

macro_rules! yank_fn {
    ($func:ident, $func_call:ident, $comment:meta $(,$n:ident)?) => {
        #[$comment]
        pub fn $func(
            doc: &mut Document,
            view: &mut Viewport,
            clipboard: &mut Clipboard,
            $($n: usize,)?
        ) -> Result<(), CommandResult> {
            let tmp_view_cur = view.cur;
            let tmp_doc_cur = doc.cur;

            cursor::$func_call(doc, view $(,$n)?);
            let res = selection(doc, &mut Some(tmp_doc_cur), clipboard);

            view.cur = tmp_view_cur;
            doc.cur = tmp_doc_cur;

            return res;
        }
    };
}

#[macro_export]
/// Convenience macro for calling yank functions. Expects a `BaseBuffer` as member `base`.
macro_rules! yank {
    ($self:ident, $func:ident) => {
        match $crate::buffer::yank::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            &mut $self.base.clipboard,
        ) {
            Ok(()) => {}
            Err(err) => return err,
        }
    };
    ($self:ident, $func:ident, REPEAT) => {{
        if let Err(err) = $crate::buffer::yank::$func(
            &mut $self.base.doc,
            &mut $self.base.doc_view,
            &mut $self.base.clipboard,
            1,
        ) {
            return err;
        }
    }};
    ($self:ident, $func:ident, SELECTION) => {
        if let Err(err) = $crate::buffer::yank::$func(
            &mut $self.base.doc,
            &mut $self.base.sel,
            &mut $self.base.clipboard,
        ) {
            return err;
        }
    };
}

/// Yanks the selected area.
pub fn selection(
    doc: &Document,
    sel: &mut Option<Cursor>,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let Some(pos) = sel else {
        return Ok(());
    };

    let res = clipboard.set_text(doc.get_range(doc.cur, *pos).unwrap());

    *sel = None;
    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::Info(err.to_string())),
    }
}

/// Yanks a line.
pub fn line(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    let start = Cursor::new(0, doc.cur.y);
    cursor::jump_to_end_of_line(doc, view);
    cursor::right(doc, view, 1);

    let res = clipboard.set_text(doc.get_range(start, doc.cur).unwrap());

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::Info(err.to_string())),
    }
}

yank_fn!(left, left, doc = "Yanks left of the cursor.", n);
yank_fn!(right, right, doc = "Yanks right of the cursor.", n);
yank_fn!(next_word, next_word, doc = "Yanks the next word.", n);
yank_fn!(prev_word, prev_word, doc = "Yanks the previous word.", n);
yank_fn!(
    next_word_end,
    next_word_end,
    doc = "Yanks to the end of the next word.",
    n
);
yank_fn!(
    prev_word_end,
    prev_word_end,
    doc = "Yanks to the end of the previous word.",
    n
);
yank_fn!(
    next_whitespace,
    next_whitespace,
    doc = "Yanks to the next whitespace.",
    n
);
yank_fn!(
    prev_whitespace,
    prev_whitespace,
    doc = "Yanks to the previous whitespace.",
    n
);
yank_fn!(
    next_empty_line,
    next_empty_line,
    doc = "Yanks to the next empty line.",
    n
);
yank_fn!(
    prev_empty_line,
    prev_empty_line,
    doc = "Yanks to the previous empty line.",
    n
);
yank_fn!(
    beginning_of_line,
    jump_to_beginning_of_line,
    doc = "Yanks until the beginning of the line."
);
yank_fn!(
    end_of_line,
    jump_to_end_of_line,
    doc = "Yanks until the end of the line."
);
yank_fn!(
    matching_opposite,
    jump_to_matching_opposite,
    doc = "Yanks until the matching opposite bracket."
);
yank_fn!(
    beginning_of_file,
    jump_to_beginning_of_file,
    doc = "Yanks until the beginning of the file."
);
yank_fn!(
    end_of_file,
    jump_to_end_of_file,
    doc = "Yanks until the end of the file."
);
