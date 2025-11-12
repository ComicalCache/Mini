use crate::{
    TXT_BUFF_IDX,
    custom_buffers::files_buffer::FilesBuffer,
    sc_buff,
    util::{CommandResult, file_name, open_file},
};
use std::{
    fs::read_dir,
    io::{Error, Read},
    path::{Path, PathBuf},
};

impl FilesBuffer {
    /// Loads a directory as path buffers and Strings. Does NOT move the cursor to be valid!
    pub(super) fn load_dir(base: &Path, entries: &mut Vec<PathBuf>) -> Result<String, Error> {
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

        let mut contents = vec!["..".to_string()];
        for entry in &mut entries.iter() {
            contents.push(if entry.is_symlink() {
                let path = entry.read_link()?;
                if path.is_dir() {
                    format!("{} -> {}/", entry.display(), path.display())
                } else {
                    format!("{} -> {}", entry.display(), path.display())
                }
            } else if entry.is_dir() {
                format!("{}/", entry.display())
            } else {
                format!("{}", entry.display())
            });
        }

        Ok(contents.join("\n"))
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
            let mut buff = String::new();
            open_file(entry)?.read_to_string(&mut buff)?;

            return Ok(sc_buff!(
                self,
                TXT_BUFF_IDX,
                buff,
                Some(entry.clone()),
                file_name(entry),
            ));
        } else if entry.is_dir() {
            self.path.clone_from(entry);
            return Ok(self.refresh());
        }

        Ok(CommandResult::Ok)
    }
}
