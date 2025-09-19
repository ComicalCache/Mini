# Mini

A mini terminal based file editor. It has VIm-like keybinds and should fully support unicode characters.
`./mini --help` prints supported operations.

It keeps the cursor "centered" around the middle 5/7th percentile, and features an info bar displaying the editing mode
, file and column numbers as well as information about the size of the buffer and if the buffer has unsafed changes.