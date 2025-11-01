use crate::{
    INFO_BUFF_IDX,
    cursor::{self, Cursor},
    document::Document,
    sc_buff,
    util::{CommandResult, split_to_lines},
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
        Err(err) => Err(sc_buff!(
            INFO_BUFF_IDX,
            split_to_lines(err.to_string()),
            None,
        )),
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

    let mut line = doc.get_range(start, doc.cur).unwrap();
    line.to_mut().push('\n');
    let res = clipboard.set_text(line);

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(sc_buff!(
            INFO_BUFF_IDX,
            split_to_lines(err.to_string()),
            None,
        )),
    }
}

yank_fn!(left, left, doc = "Yanks left of the cursor.", n);
yank_fn!(right, right, doc = "Yanks right of the cursor.", n);
yank_fn!(next_word, next_word, doc = "Yanks the next word.", n);
yank_fn!(prev_word, prev_word, doc = "Yanks the previous word.", n);
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
