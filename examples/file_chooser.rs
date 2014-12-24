extern crate file_dialog;
extern crate sdl2_window;
extern crate opengl_graphics;

use file_dialog::{FileDialog, SelectType};
use sdl2_window::Sdl2Window;
use opengl_graphics::OpenGL;
use opengl_graphics::glyph_cache::GlyphCache as Font;

use std::borrow::ToOwned;

fn main() {
    let promise = FileDialog::new("File Dialog Test", font())
        .show(Sdl2Window::new, OpenGL::_3_2);
       
    if let Some(file) = promise.join().unwrap_or(None) {
        println!("Selected file: {}", file.display());
    }

    let promise = FileDialog::new("File Save Test", font())
        .set_select(SelectType::SaveFile(Some("filename.txt".to_owned())))
        .show(Sdl2Window::new, OpenGL::_3_2);

    if let Some(file) = promise.join().unwrap_or(None) {
        println!("Selected file: {}", file.display());
    }
}

fn font() -> Font {
    Font::new(&Path::new("./assets/Dense-Regular.otf")).unwrap()
}
