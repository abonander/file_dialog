file_dialog
===========

A file dialog in Rust using Conrod. Supports file/folder selection, and file saving. Does *not* modify the filesystem, only returning `Path`. You should confirm the returned paths are valid before operating on them.

See `examples/file_chooser.rs` for how to invoke the dialog.

####Note


While `file_dialog` is polymorphic over different `Window` implementations, `GlfwWindow` is currently not very useful as it cannot open more than one window at a time. The example had to be changed to use `Sdl2Window` for this reason.
