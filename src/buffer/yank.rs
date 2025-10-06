use crate::{
    INFO_BUFF_IDX,
    cursor::{self, Cursor},
    document::Document,
    util::CommandResult,
    viewport::Viewport,
};
use arboard::Clipboard;
use std::borrow::Cow;

/// Yanks the selected area.
pub fn selection(
    doc: &mut Document,
    sel: &mut Option<Cursor>,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let Some(pos) = sel else {
        return Ok(());
    };

    let res = clipboard.set_text(doc.get_range(doc.cur, *pos).expect("Illegal state"));

    *sel = None;
    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
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

    let mut line = doc.get_range(start, doc.cur).expect("Illegal state");
    line.to_mut().push('\n');
    let res = clipboard.set_text(line);

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}

/// Yanks left of the cursor.
pub fn left(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    cursor::left(doc, view, 1);
    let res = clipboard.set_text(doc.get_range(tmp_doc_cur, doc.cur).expect("Illegal state"));

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}

/// Yanks right of the cursor.
pub fn right(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    cursor::right(doc, view, 1);
    let res = clipboard.set_text(doc.get_range(tmp_doc_cur, doc.cur).expect("Illegal state"));

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}

/// Yanks the next word.
pub fn next_word(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    cursor::next_word(doc, view, 1);
    let res = clipboard.set_text(doc.get_range(tmp_doc_cur, doc.cur).expect("Illegal state"));

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}

/// Yanks the previous word.
pub fn prev_word(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    cursor::prev_word(doc, view, 1);
    let res = clipboard.set_text(doc.get_range(tmp_doc_cur, doc.cur).expect("Illegal state"));

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}

/// Yanks until the beginning of the line.
pub fn beginning_of_line(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    cursor::jump_to_beginning_of_line(doc, view);
    let res = clipboard.set_text(doc.get_range(tmp_doc_cur, doc.cur).expect("Illegal state"));

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}

/// Yanks until the end of the line.
pub fn end_of_line(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    cursor::jump_to_end_of_line(doc, view);
    let res = clipboard.set_text(doc.get_range(tmp_doc_cur, doc.cur).expect("Illegal state"));

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}

/// Yanks until the matching opposite bracket.
pub fn matching_opposite(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    cursor::jump_to_matching_opposite(doc, view);
    let res = clipboard.set_text(doc.get_range(tmp_doc_cur, doc.cur).expect("Illegal state"));

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}

/// Yanks until the beginning of the file.
pub fn beginning_of_file(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    cursor::jump_to_beginning_of_file(doc, view);
    let res = clipboard.set_text(doc.get_range(tmp_doc_cur, doc.cur).expect("Illegal state"));

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}

/// Yanks until the end of the file.
pub fn end_of_file(
    doc: &mut Document,
    view: &mut Viewport,
    clipboard: &mut Clipboard,
) -> Result<(), CommandResult> {
    let tmp_view_cur = view.cur;
    let tmp_doc_cur = doc.cur;

    cursor::jump_to_end_of_file(doc, view);
    let res = clipboard.set_text(doc.get_range(tmp_doc_cur, doc.cur).expect("Illegal state"));

    view.cur = tmp_view_cur;
    doc.cur = tmp_doc_cur;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(CommandResult::SetAndChangeBuffer(
            INFO_BUFF_IDX,
            vec![Cow::from(err.to_string())],
            None,
        )),
    }
}
