file_dialog
===========

A file dialog in Rust using Conrod. Supports file/folder selection, and file saving. Does *not* modify the filesystem, only returning `Path`. You should confirm the returned paths are valid before operating on them.

See `examples/file_chooser.rs` for how to invoke the dialog.

####Note


While `file_dialog` is polymorphic over different `Window` implementations, `GlfwWindow` is currently not very useful as it cannot open more than one window at a time. The example had to be changed to use `Sdl2Window` for this reason.

###TODO
* Add documentation comments
* RustCI and TravisCI integration
* Improve examples
    * Find better font
    * Add folder selection example
* Improve dialog design
    * Fix alignments
    * Make positioning smarter
    * Support custom resolutions/themes
* Make `FilePromise` safer (non-panicking)

Screenshots
===========
It's not pretty, but it works and that's all that matters. Design can be fixed later.
####Selecting File
![][select-file]

####Saving File
![][saving-file]

[select-file]: http://i.imgur.com/YYlAMbn.png
[saving-file]: http://i.imgur.com/SZekC2Y.png
