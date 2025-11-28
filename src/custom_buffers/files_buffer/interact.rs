use crate::{
    custom_buffers::{files_buffer::FilesBuffer, text_buffer::TextBuffer},
    util::{Command, file_name, open_file},
};
use std::{
    fs::read_dir,
    io::Error,
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
    pub(super) fn select_item(&mut self) -> Result<Command, Error> {
        let idx = self.base.doc.cur.y;

        // Move directory up.
        if idx == 0 {
            if self.path.pop() {
                return Ok(self.refresh());
            }

            return Ok(Command::Ok);
        }

        let entry = &self.entries[idx.saturating_sub(1)].clone();
        if entry.is_file() {
            let text_buffer = TextBuffer::new(
                self.base.w,
                self.base.h,
                self.base.x_off,
                self.base.y_off,
                Some(open_file(entry)?),
                file_name(entry),
            )?;

            // Replace this `FilesBuffer` instance with a `TextBuffer` instance containing the file content.
            return Ok(Command::Init(Box::new(text_buffer)));
        } else if entry.is_dir() {
            self.path.clone_from(entry);
            return Ok(self.refresh());
        }

        Ok(Command::Ok)
    }
}
