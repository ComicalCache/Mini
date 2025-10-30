use crate::{
    INFO_BUFF_IDX,
    cursor::{self, Cursor},
    document::Document,
    sc_buff,
    util::{CommandResult, split_to_lines},
    viewport::Viewport,
};
use arboard::Clipboard;

macro_rules! yank {
    ($doc:ident, $view:ident, $clipboard:ident, $func:ident $(,$n:ident)?) => {{
        let tmp_view_cur = $view.cur;
        let tmp_doc_cur = $doc.cur;

        cursor::$func($doc, $view $(,$n)?);
        let res = selection($doc, &mut Some(tmp_doc_cur), $clipboard);

        $view.cur = tmp_view_cur;
        $doc.cur = tmp_doc_cur;

        return res;
    }};
}

/// Yanks the selected area.
pub fn selection(
    doc: &mut Document,
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

/// Yanks left of the cursor.
pub fn left(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
    n: usize,
) -> Result<(), CommandResult> {
    yank!(doc, view, clipboard, left, n)
}

/// Yanks right of the cursor.
pub fn right(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
    n: usize,
) -> Result<(), CommandResult> {
    yank!(doc, view, clipboard, right, n)
}

/// Yanks the next word.
pub fn next_word(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
    n: usize,
) -> Result<(), CommandResult> {
    yank!(doc, view, clipboard, next_word, n)
}

/// Yanks the previous word.
pub fn prev_word(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
    n: usize,
) -> Result<(), CommandResult> {
    yank!(doc, view, clipboard, prev_word, n)
}

/// Yanks until the beginning of the line.
pub fn beginning_of_line(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    yank!(doc, view, clipboard, jump_to_beginning_of_line)
}

/// Yanks until the end of the line.
pub fn end_of_line(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    yank!(doc, view, clipboard, jump_to_end_of_line)
}

/// Yanks until the matching opposite bracket.
pub fn matching_opposite(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    yank!(doc, view, clipboard, jump_to_matching_opposite)
}

/// Yanks until the beginning of the file.
pub fn beginning_of_file(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    yank!(doc, view, clipboard, jump_to_beginning_of_file)
}

/// Yanks until the end of the file.
pub fn end_of_file(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    yank!(doc, view, clipboard, jump_to_end_of_file)
}
