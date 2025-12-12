# Mini

A "minimalistic" terminal based text-editor.

### Features
Mini supports ergonomic motions, text selection, search and replace, undo and redo, a file browser, opening multiple
buffers, running terminal commands, wide-character texts and more.

`./mini --help` prints an exhaustiv documentation of all features. See `info.txt` for the command output or type space,
`?` and hit enter to see it inside the editor.

### Installation
Mini can simply be built with `cargo build --release` or installed via cargo with `cargo install --path .`.

### Interface
The editor features an info line containing information about the current buffer, editing mode, cursor position, etc..
![Screenshot in editor](https://github.com/ComicalCache/Mini/blob/main/media/editor.png?raw=true)

Open multiple buffer and switch between them.
![Screenshot in editor](https://github.com/ComicalCache/Mini/blob/main/media/buffer.png?raw=true)

The files in the current folder can be browsed and opened from within Mini too!
![Screenshot of the file browser](https://github.com/ComicalCache/Mini/blob/main/media/files.png?raw=true)

### Thanks
Thanks to [Ted Mielczarek](https://github.com/luser) who's [strip-ansi-escapes](https://github.com/luser/strip-ansi-escapes/tree/master) I slightly modified and is used in `src/shell_command`.
