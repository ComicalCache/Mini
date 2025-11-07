use crate::{
    TXT_BUFF_IDX, cursor,
    custom_buffers::files_buffer::FilesBuffer,
    sc_buff,
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
        base: &Path,
        entries: &mut Vec<PathBuf>,
    ) -> Result<Vec<Cow<'static, str>>, Error> {
        let mut base = if base.is_dir() {
            base.to_path_buf()
        } else {
            PathBuf::from(base.parent().unwrap_or_else(|| Path::new("")))
        };
        if !base.exists() {
            base = std::env::current_dir()?;
        }

        *entries = read_dir(base)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, Error>>()?;
        entries.sort();

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
        if idx == 0 {
            if self.path.pop() {
                self.base
                    .doc
                    .set_contents(&Self::load_dir(&self.path, &mut self.entries)?);
            }

            return Ok(CommandResult::Ok);
        }

        let entry = &self.entries[idx.saturating_sub(1)].clone();
        if entry.is_file() {
            self.path.clone_from(entry);

            return Ok(sc_buff!(
                TXT_BUFF_IDX,
                read_file_to_lines(&mut open_file(entry)?)?,
                Some(self.path.clone()),
                Path::new(entry)
                    .file_name()
                    .map(|p| p.to_string_lossy().to_string()),
            ));
        } else if entry.is_dir() {
            self.path.clone_from(entry);

            cursor::jump_to_beginning_of_file(&mut self.base.doc, &mut self.base.doc_view);
            self.base
                .doc
                .set_contents(&Self::load_dir(&self.path, &mut self.entries)?);
            return Ok(CommandResult::Ok);
        } else if entry.is_symlink() && entry.exists() {
            // FIXME: this can cause an infinite loop on a circular symlink.
            self.path = entry.read_link()?;
            return self.select_item();
        }

        Ok(CommandResult::Ok)
    }
}
