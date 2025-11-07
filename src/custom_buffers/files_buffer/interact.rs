use crate::{
    TXT_BUFF_IDX,
    custom_buffers::files_buffer::FilesBuffer,
    sc_buff,
    util::{CommandResult, file_name, open_file, read_file_to_lines},
};
use std::{
    borrow::Cow,
    fs::read_dir,
    io::Error,
    path::{Path, PathBuf},
};

impl FilesBuffer {
    /// Loads a directory as path buffers and Strings. Does NOT move the cursor to be valid!
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
                return Ok(self.refresh());
            }

            return Ok(CommandResult::Ok);
        }

        let entry = &self.entries[idx.saturating_sub(1)].clone();
        if entry.is_file() {
            return Ok(sc_buff!(
                TXT_BUFF_IDX,
                read_file_to_lines(&mut open_file(entry)?)?,
                Some(entry.clone()),
                file_name(entry)
            ));
        } else if entry.is_dir() {
            self.path.clone_from(entry);
            return Ok(self.refresh());
        }

        Ok(CommandResult::Ok)
    }
}
