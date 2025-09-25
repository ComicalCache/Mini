use std::{
    fs::{File, OpenOptions},
    io::Error,
    path::Path,
};

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<File, Error> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}
