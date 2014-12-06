#![feature(unboxed_closures, macro_rules, globs)]

extern crate shader_version;
extern crate event;
extern crate event_loop;
extern crate conrod;
extern crate graphics;
extern crate window;
extern crate opengl_graphics;
extern crate current;

use conrod::*;

use current::Set;

use event::Event;
use event_loop::{Events, Ups, MaxFps};

use opengl_graphics::Gl;
use opengl_graphics::glyph_cache::GlyphCache as Font;

use shader_version::opengl::OpenGL;

use window::{Window, WindowSettings};

use std::comm::TryRecvError;

use std::default::Default;

use std::io::fs::{mod, PathExtensions};
use std::io::IoResult;

use std::os;

use std::cell::RefCell;

pub struct FileDialog {
    title: String,
    dimen: [u32, ..2],
    samples: u8,
    background: Color,
    select: SelectType,
    starting_path: Path,
    font: Font,
}

macro_rules! setter(
    ($field:ident: $ty:ty -> $ret:ident) => ( 
        fn $field(mut self, $field: $ty) -> $ret {
            self.$field = $field;
            self
        }
    );
)

impl FileDialog {
    pub fn new<S: StrAllocating>(title: S, font: Font) -> FileDialog {
        FileDialog {
            title: title.into_string(),
            dimen: [640, 480],
            samples: 4,
            background: Color::new(0.9, 0.9, 0.9, 1.0), // Should be a nice light-grey
            select: SelectType::File,
            // Possible panic! here, but unlikely.
            starting_path: os::homedir().unwrap_or_else(|| os::getcwd().unwrap()),
            font: font,
        }
    }

    pub fn width(mut self, width: u32) -> FileDialog {
        self.dimen[0] = width;
        self   
    }

    pub fn height(mut self, height: u32) -> FileDialog {
        self.dimen[1] = height;
        self  
    }

    pub fn dimensions(mut self, width: u32, height: u32) -> FileDialog {
        self.dimen = [width, height];
        self    
    }

    setter!(samples: u8 -> FileDialog)
    
    setter!(background: Color -> FileDialog)
    
    setter!(select: SelectType -> FileDialog)

    setter!(starting_path: Path -> FileDialog) 
    
    // How should we format the trait bounds here?
    /// Show the dialog
    pub fn show<W: Window, F: FnOnce(OpenGL, WindowSettings) -> W + Send>
    (self, win_fn: F, gl: OpenGL) -> FilePromise {
        let (dialog, window) = self.explode();        
        let (promise, tx) = FilePromise::new();

        spawn(proc() render_file_dialog(dialog, window, gl, win_fn, tx));
         
        promise                            
    }

    fn explode(self) -> (DialogSettings, WindowSettings) {
        (
            DialogSettings {
                background: self.background,
                select: self.select,
                starting_path: self.starting_path,
                font: self.font,
            },
            WindowSettings {
                title: self.title,
                size: self.dimen,
                samples: self.samples,
                fullscreen: false,
                exit_on_esc: true,
            }
        )    
    }
}

/// An enum describing the file selection behavior.
pub enum SelectType {
    /// User must select an existing file on the filesystem.
    File,
    /// User must select an existing folder on the filesystem, or create a new one.
    Folder,
    /// User must select the location to save a file, and the filename.
    /// The contained `Option<String>` is the default filename, if any.
    SaveFile(Option<String>),    
}

impl SelectType {
    fn show_files(&self) -> bool {
        match *self {
            SelectType::File | SelectType::SaveFile(_) => true,
            _ => false,
        }    
    }    
}

struct DialogSettings {
    background: Color,
    select: SelectType,
    starting_path: Path,
    font: Font,
}

impl DialogSettings {
    fn into_state(self) -> (DialogState, Font) {
        (
            DialogState {
                dir: self.starting_path,
                selected: None,
                folders: Vec::new(),
                files: Vec::new(),
                background: self.background,
                select: self.select,             
            },
            self.font,
        )
    }    
}

fn render_file_dialog<W: Window, F: FnOnce(OpenGL, WindowSettings) -> W>
(dialog: DialogSettings, window: WindowSettings, gl: OpenGL, win_fn: F, tx: Sender<Path>) {
    let (mut state, font) = dialog.into_state();

    let ref mut state = state;

    let window = win_fn(gl, window);
    let mut event_loop = Events::new(window).set(Ups(120)).set(MaxFps(60));
    let mut gl = Gl::new(gl);
    
    let ref mut uic = UiContext::new(font, Theme::default());
    let ref mut buf = Default::default();

    for event in event_loop {
        uic.handle_event(&event);
        match event {
            Event::Render(args) => {
                gl.draw([0, 0, args.width as i32, args.height as i32], |_, gl| {
                    draw_dialog_ui(gl, uic, state, buf);
                });
            },
            _ => {}    
        }
    }
}

#[deriving(Default)]
struct Buffers {
    dir: String,    
}

struct DialogState {
    dir: Path,
    selected: Option<Path>,
    folders: Vec<Path>,
    files: Vec<Path>,
    background: Color,
    select: SelectType,
}


impl DialogState {
    fn update_path(&mut self, new_dir: &str) -> bool {
        let path = Path::new(new_dir);

        if path.is_dir() {
            self.dir = path;

            self.folders = list_folders(&self.dir).unwrap();
            
            if self.select.show_files() {
                self.files = list_files(&self.dir).unwrap();
            } else {
                self.files = Vec::new();
            }

            true
        } else {
            false
        }
    }
}


fn draw_dialog_ui(gl: &mut Gl, uic: &mut UiContext, state: &mut DialogState, buf: &mut Buffers) {
    uic.background().color(state.background).draw(gl);

    if buf.dir.is_empty() {
        state.dir.as_str().map(|s| buf.dir.push_str(s));
    }

    uic.text_box(42, &mut buf.dir)
        .font_size(24u32)
        .dimensions(270.0, 30.0)
        .callback(|new_dir: &mut String| {
            if !state.update_path(&**new_dir) {
                new_dir.clear(); 
                state.dir.as_str().map(|s| new_dir.push_str(s));
            }
        })
        .position(30.0, 30.0)
        .draw(gl);
}

fn list_folders(path: &Path) -> IoResult<Vec<Path>> {
    let mut entries = try!(fs::readdir(path));
    entries.retain(|file| file.is_dir());    
    Ok(entries)
} 

fn list_files(path: &Path) -> IoResult<Vec<Path>> {
    let mut entries = try!(fs::readdir(path));
    entries.retain(|file| !file.is_dir());
    Ok(entries)
}

pub struct FilePromise {
    opt: Option<Path>,
    rx: Receiver<Path>,
}

impl FilePromise {
    pub fn new() -> (FilePromise, Sender<Path>) {
        let (tx, rx) = channel();

        (
            FilePromise {
                opt: None,
                rx: rx,
            }, 
            tx,
        )            
    }    
    
    pub fn poll(&mut self) -> Option<&Path> {
        match self.rx.try_recv() {
            Ok(val) => self.opt = Some(val),
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) if self.opt.is_none() => 
                panic!("Promised value never received; processing task ended prematurely!"),
            _ => (),
        }                         

        self.opt.as_ref()
    }
    
    pub fn unwrap(mut self) -> Path {
        while self.poll().is_none() {} 
        
        self.opt.unwrap()   
    } 
}
