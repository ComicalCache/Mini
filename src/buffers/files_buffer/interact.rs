use std::{borrow::Cow, fs::read_dir, io::Error, path::PathBuf};

use crate::{
    TXT_BUFF_IDX,
    buffers::files_buffer::FilesBuffer,
    cursor_move as cm,
    util::{CommandResult, open_file, read_file_to_lines},
};

impl FilesBuffer {
    /// Loads a directory as path buffers and Strings.
    pub(super) fn load_dir(
        base: &PathBuf,
        entries: &mut Vec<PathBuf>,
        contents: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), Error> {
        let mut tmp_entries = read_dir(base)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, Error>>()?;
        tmp_entries.sort();
        *entries = tmp_entries;

        let mut tmp_contents = vec![Cow::from("..")];
        tmp_contents.append(
            &mut entries
                .iter()
                .map(|p| Cow::from(p.to_string_lossy().to_string()))
                .collect(),
        );
        *contents = tmp_contents;

        Ok(())
    }

    /// Handles the user selection of an entry in the file buffer.
    pub(super) fn select_item(&mut self) -> Result<CommandResult, Error> {
        let idx = self.doc.cur.y;

        // Move directory up.
        if idx == 0 && self.base.pop() {
            cm::jump_to_beginning_of_file(&mut self.doc, &mut self.view);

            FilesBuffer::load_dir(&self.base, &mut self.entries, &mut self.doc.buff)?;
            return Ok(CommandResult::Ok);
        }

        let entry = &self.entries[idx - 1].clone();
        if entry.is_file() {
            cm::jump_to_beginning_of_file(&mut self.doc, &mut self.view);
            let contents = read_file_to_lines(&mut open_file(entry)?)?;
            self.base.clone_from(entry);

            return Ok(CommandResult::SetAndChangeBuffer(
                TXT_BUFF_IDX,
                contents,
                Some(self.base.clone()),
            ));
        } else if entry.is_dir() {
            cm::jump_to_beginning_of_file(&mut self.doc, &mut self.view);
            self.base.clone_from(entry);

            FilesBuffer::load_dir(&self.base, &mut self.entries, &mut self.doc.buff)?;
            return Ok(CommandResult::Ok);
        } else if entry.is_symlink() {
            // TODO: handle Symlinks.
            return Ok(CommandResult::Ok);
        }

        Ok(CommandResult::Ok)
    }
}
