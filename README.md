# Mini

A mini terminal based file editor. It has VIm-like motions and should fully support unicode characters.

### Interface
The editor features an info line containing information about the current buffer, editing mode, cursor position, etc..
![Screenshot in editor](https://github.com/ComicalCache/Mini/blob/main/media/editor.png?raw=true)

Infos like errors and help are shown in a different buffer, reserved for infos.
![Screenshot of the help message](https://github.com/ComicalCache/Mini/blob/main/media/info.png?raw=true)

The files in the current folder can be browsed and opened from within Mini too!
![Screenshot of the file browser](https://github.com/ComicalCache/Mini/blob/main/media/files.png?raw=true)

### Features
`./mini --help` prints supported operations. See the `src/info.txt` for the command output or type `?` in command mode to see it inside the editor.
