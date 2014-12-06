extern crate file_dialog;
extern crate glfw_window;
extern crate shader_version;
extern crate opengl_graphics;

use file_dialog::FileDialog;
use glfw_window::GlfwWindow;
use opengl_graphics::glyph_cache::GlyphCache as Font;
use shader_version::opengl::OpenGL;

fn main() {
    let font = Font::new(&Path::new("./assets/Dense-Regular.otf")).unwrap(); 

    let promise = FileDialog::new("File Dialog Test", font)
        .show(GlfwWindow::new, OpenGL::OpenGL_3_2);
       
    promise.unwrap(); 
}
