# Mini

A mini terminal based file editor. It has VIm-like motions and should fully support unicode characters.
> The editor only respects terminal resizing at the next input.

### Interface
The editor features an info line containing information about the current buffer, editing mode, cursor position, etc..
![Screenshot in editor](https://github.com/ComicalCache/Mini/blob/main/media/editor.png?raw=true)

Errors are shown in a different buffer, reserved for errors which can be switched to.
![Screenshot showing an error](https://github.com/ComicalCache/Mini/blob/main/media/error.png?raw=true)

Commands (not motions) can be written after entering command mode.
![Screenshot showing the command input](https://github.com/ComicalCache/Mini/blob/main/media/command.png?raw=true)

### Features
`./mini --help` prints supported operations.
```
> ./mini --help
 Mini terminal text editor, run with file(path) argument to open or create

   Type a number followed by a motion to execute it multiple times
   Press h | j | k | l to move the cursor
   Press w to skip to the next word
   Press b to go back one word
   Press < | > to jump to the beginning/end of a line
   Press . to jump to the matching opposite bracket
   Press e to switch between the error and text buffer
   Press space to enter command mode
     Write q to quit
     Write w to write the buffer to file
     Write w <path> to write this/all future writes to the specified path
     Write o <path> to open a file and replace the buffer
     Write oo <path> to open a file and replace the buffer, discarding unsaved changes
     Press esc to exit command mode
   Press v to start selection at the current cursor position
     Move cursor and press d to delete the selection
     Press esc to stop selection
   Press i to enter write mode
   Press a to enter write mode one character after the current
   Press o to enter write mode one line under the current
   Press O to enter write mode one line above the current
   Press g to go to the end of the file
   Press G to go to the start of the file
   Press esc to exit write mode
```
