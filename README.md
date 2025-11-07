# Mini

A "minimalistic" terminal based text-editor. It has VIm-like motions and should fully support unicode characters.

### Features
Mini supports ergonomic motions, text selection, search and replace, a file browser and more.

`./mini --help` prints an exhaustiv documentation of all features. See the `src/info.txt` for the command output or type space, `?` and hit enter to see it inside the editor.

### Installation
Mini can simply be built with `cargo build --release` or installed via cargo with `cargo install --path .`.

### Cargo Features
- `syntax-highlighting`: This enables the syntax highlighting capabilities of Mini (this causes the binary to be much
  much larger due to the grammar files)

### Interface
The editor features an info line containing information about the current buffer, editing mode, cursor position, etc..
![Screenshot in editor](https://github.com/ComicalCache/Mini/blob/main/media/editor.png?raw=true)

Infos like errors and help are shown in a different buffer, reserved for infos.
![Screenshot of the help message](https://github.com/ComicalCache/Mini/blob/main/media/info.png?raw=true)

The files in the current folder can be browsed and opened from within Mini too!
![Screenshot of the file browser](https://github.com/ComicalCache/Mini/blob/main/media/files.png?raw=true)
