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
    filter_hidden: bool,
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
            filter_hidden: true,
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
    
    setter!(filter_hidden: bool -> FileDialog) 
    
    // How should we format the trait bounds here?
    /// Show the dialog
    pub fn show<W: Window, F: FnOnce(OpenGL, WindowSettings) -> W + Send>
    (self, win_fn: F, gl: OpenGL) -> FilePromise {
        let (promise, tx) = FilePromise::new();
        let (dialog, window) = self.explode(tx);        

        spawn(proc() render_file_dialog(dialog, window, gl, win_fn));
         
        promise                            
    }

    fn explode(self, tx: Sender<Path>) -> (DialogSettings, WindowSettings) {
        (
            DialogSettings {
                background: self.background,
                select: self.select,
                starting_path: self.starting_path,
                font: self.font,
                tx: tx,
                filter_hidden: self.filter_hidden,
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
    tx: Sender<Path>,
    filter_hidden: bool,
}

impl DialogSettings {
    fn into_state(self) -> (DialogState, Font) {
        (
            DialogState {
                dir: self.starting_path,
                selected: None,
                paths: Vec::new(),
                background: self.background,
                select: self.select,
                dir_changed: false,
                pages: 0,
                tx: self.tx,
                sent: false,
                filter_hidden: self.filter_hidden,
            },
            self.font,
        )
    }    
}

fn render_file_dialog<W: Window, F: FnOnce(OpenGL, WindowSettings) -> W>
(dialog: DialogSettings, window: WindowSettings, gl: OpenGL, win_fn: F) {
    let (mut state, font) = dialog.into_state();
    state.update_paths();

    let ref mut state = state;

    let window = win_fn(gl, window);
    let mut event_loop = Events::new(window).set(Ups(120)).set(MaxFps(60));
    let mut gl = Gl::new(gl);
    
    let ref mut uic = UiContext::new(font, Theme::default());
    let ref mut buf = Default::default();

    for event in event_loop {
        if state.sent { return; }

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
    selected: Option<uint>,
    paths: Vec<Path>,
    background: Color,
    select: SelectType,
    dir_changed: bool,
    pages: uint,
    tx: Sender<Path>,
    sent: bool,
    filter_hidden: bool,
}

impl DialogState {
    fn update_dir(&mut self, new_dir: Path) {
        self.dir_changed = if new_dir.is_dir() {
            self.dir = new_dir;
            self.update_paths();
            true
        } else {
            false
        }
    }

    fn update_paths(&mut self) {
        self.selected = None;
        self.paths = entries(&self.dir, self.select.show_files(), self.filter_hidden).unwrap();

        let count = self.paths.len();
        let per_page = COLS * ROWS;
        self.pages = count / per_page;

        if count % per_page != 0 { self.pages += 1; }
    }

    fn up_dir(&mut self) {
        self.dir_changed = self.dir.pop();
        self.update_paths();
    }

    fn select(&mut self, num: uint) {
        // Double-clicked
        if self.selected == Some(num) {
            let path = self.paths.remove(num).unwrap();
            
            if self.select.show_files() && !path.is_dir() {
                self.tx.send(path);
                self.sent = true;    
            } else {
                self.update_dir(path);
            }            
        } else {
            self.selected = Some(num);
        }        
    }
}

const COLS: uint = 5;
const ROWS: uint = 10;

fn draw_dialog_ui(gl: &mut Gl, uic: &mut UiContext, state: &mut DialogState, buf: &mut Buffers) {
    uic.background().color(state.background).draw(gl);

    if state.dir_changed {
        buf.dir.clear();
    }

    if buf.dir.is_empty() {
        state.dir.as_str().map(|s| buf.dir.push_str(s));
    }   
        
    let text_id = 42u64;

    const BUTTON_ID: u64 = 78;
    
    uic.button(BUTTON_ID)
        .dimensions(30.0, 30.0)
        .position(605.0, 5.0)
        .label("Up")
        .callback(|| state.up_dir())
        .draw(gl);

    uic.label(&*buf.dir)
        .position(5.0, 5.0)
        .size(24)
        .draw(gl);
    
    let base_id = 96u64;

    uic.widget_matrix(COLS, ROWS)
        .position(5.0, 35.0)
        .dimensions(635.0, 440.0)
        .cell_padding(5.0, 5.0)
        .each_widget(|uic, num, _, _, pt, dimen| {
            if num >= state.paths.len() { return; }

            let label = state.paths[num].filename_str().unwrap().into_string();

            let button = uic.button(base_id + num as u64)
                .point(pt)
                .dimensions(dimen[0], dimen[1]) 
                .label(&*label).label_font_size(20);

                if state.selected == Some(num) {
                    button.color(Color::new(0.5, 0.9, 0.5, 1.0))
                } else { button }
                .callback(|| state.select(num))
                .draw(gl);                
        });
}

fn entries(path: &Path, keep_files: bool, filter_hidden: bool) -> IoResult<Vec<Path>> {
    let mut entries = try!(fs::readdir(path));
    entries.retain(|file|
        (filter_hidden && !file.filename_str().unwrap().starts_with(".")) &&
        (keep_files || file.is_dir())
    );
    entries.sort();

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
