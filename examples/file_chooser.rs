extern crate file_dialog;
extern crate sdl2_window;
extern crate opengl_graphics;

use file_dialog::{FileDialog, SelectType};
use sdl2_window::Sdl2Window;
use opengl_graphics::OpenGL;
use opengl_graphics::glyph_cache::GlyphCache as Font;

fn main() {
    let promise = FileDialog::new("File Dialog Test", font())
        .show(Sdl2Window::new, OpenGL::_3_2);
       
    println!("Selected file: {}", promise.unwrap().display());

    let promise = FileDialog::new("File Save Test", font())
        .set_select(SelectType::SaveFile(Some("filename.txt".into_string())))
        .show(Sdl2Window::new, OpenGL::_3_2);

    println!("Selected file: {}", promise.unwrap().display());
}

fn font() -> Font {
    Font::new(&Path::new("./assets/Dense-Regular.otf")).unwrap()
}
