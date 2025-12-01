use crate::shell_command::writer::Writer;
use std::io::Write;

/// Strip ANSI escapes from `data` and return the remaining bytes as a `Vec<u8>`.
pub fn strip<T: AsRef<[u8]>>(data: T) -> Vec<u8> {
    let mut writer = Writer::new(Vec::new());
    writer.write_all(data.as_ref()).unwrap();
    writer.into_inner().unwrap()
}

/// Strip ANSI escapes from `data` and return the remaining contents as a `String`.
pub fn strip_str<T: AsRef<str>>(data: T) -> String {
    let data = data.as_ref().replace("\r\n", "\n");
    String::from_utf8(strip(data.as_bytes())).unwrap()
}
