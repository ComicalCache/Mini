use crate::{
    TXT_BUFF_IDX,
    cursor::{self, Cursor},
    custom_buffers::files_buffer::FilesBuffer,
    util::{CommandResult, open_file, read_file_to_lines},
};
use std::{
    borrow::Cow,
    fs::read_dir,
    io::Error,
    path::{Path, PathBuf},
};

impl FilesBuffer {
    /// Loads a directory as path buffers and Strings.
    pub(super) fn load_dir(
        base: &PathBuf,
        entries: &mut Vec<PathBuf>,
    ) -> Result<Vec<Cow<'static, str>>, Error> {
        let base = if base.is_dir() {
            base
        } else {
            &PathBuf::from(base.parent().unwrap_or(Path::new("/")))
        };

        let mut tmp_entries = read_dir(base)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, Error>>()?;
        tmp_entries.sort();
        *entries = tmp_entries;

        let mut contents = vec![Cow::from("..")];
        for entry in &mut entries.iter() {
            contents.push(if entry.is_symlink() {
                let path = entry.read_link()?;
                if path.is_dir() {
                    Cow::from(format!("{} -> {}/", entry.display(), path.display()))
                } else {
                    Cow::from(format!("{} -> {}", entry.display(), path.display()))
                }
            } else if entry.is_dir() {
                Cow::from(format!("{}/", entry.display()))
            } else {
                Cow::from(format!("{}", entry.display()))
            });
        }

        Ok(contents)
    }

    /// Handles the user selection of an entry in the file buffer.
    pub(super) fn select_item(&mut self) -> Result<CommandResult, Error> {
        let idx = self.base.doc.cur.y;

        // Move directory up.
        if idx == 0 && self.path.pop() {
            self.base.doc.set_contents(
                &FilesBuffer::load_dir(&self.path, &mut self.entries)?,
                0,
                0,
            );
            return Ok(CommandResult::Ok);
        }

        let entry = &self.entries[idx.saturating_sub(1)].clone();
        if entry.is_file() {
            cursor::jump_to_beginning_of_file(&mut self.base.doc, &mut self.base.doc_view);
            let contents = read_file_to_lines(&mut open_file(entry)?)?;
            self.path.clone_from(entry);

            return Ok(CommandResult::SetAndChangeBuffer(
                TXT_BUFF_IDX,
                contents,
                Some(self.path.clone()),
            ));
        } else if entry.is_dir() {
            self.path.clone_from(entry);

            self.base.doc.set_contents(
                &FilesBuffer::load_dir(&self.path, &mut self.entries)?,
                0,
                0,
            );
            self.base.doc_view.cur = Cursor::new(0, 0);
            return Ok(CommandResult::Ok);
        } else if entry.is_symlink() && entry.exists() {
            // FIXME: this can cause an infinite loop on a circular symlink.
            self.path = entry.read_link()?;
            return self.select_item();
        }

        Ok(CommandResult::Ok)
    }
}
