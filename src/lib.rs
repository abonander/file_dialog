#![feature(unboxed_closures, macro_rules)]

extern crate shader_version;
extern crate event;
extern crate conrod;
extern crate graphics;
extern crate window;
extern crate opengl_graphics;
extern crate current;

use conrod::Color;

use shader_version::opengl::OpenGL;

use window::{Window, WindowSettings};

use std::comm::TryRecvError;

use std::io::fs::{mod, PathExtensions};
use std::io::IoResult;

use std::os;

pub struct FileDialog {
    title: String,
    dimen: [u32, ..2],
    samples: u8,
    background: Color,
    select: SelectType,
    starting_path: Path,
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
    pub fn with_title<S: StrAllocating>(title: S) -> FileDialog {
        FileDialog {
            title: title.into_string(),
            dimen: [0, ..2],
            samples: 4,
            background: Color::new(0.9, 0.9, 0.9, 1.0), // Should be a nice light-grey
            select: SelectType::File,
            // Possible panic! here, but unlikely.
            starting_path: os::homedir().unwrap_or_else(|| os::getcwd().unwrap())
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
    (self, win_fn: F) -> FilePromise {
        let (dialog, window_settings) = self.explode();        
        let (promise, tx) = FilePromise::new();

        spawn(proc() render_file_dialog(dialog, win_fn, tx));
         
        promise                            
    }

    fn explode(self) -> (DialogSettings, WindowSettings) {
        (
            DialogSettings {
                background: self.background,
                select: self.select,
                starting_path: self.starting_path,
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

struct DialogSettings {
    background: Color,
    select: SelectType,
    starting_path: Path,   
}

fn render_file_dialog<W: Window, F: FnOnce(OpenGL, WindowSettings) -> W>
(settings: DialogSettings, win_fn: F, tx: Sender<Path>) {
    unimplemented!();        
}

fn list_folders(path: &Path) -> IoResult<Vec<Path>> {
    let mut entries = try!(fs::readdir(path));
    entries.retain(PathExtensions::is_dir);    
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
                panic!("Promised value never received; processing task panicked!"),
            _ => (),
        }                         

        self.opt.as_ref()
    }
    
    pub fn unwrap(mut self) -> Path {
        while self.poll().is_none() {} 
        
        self.opt.unwrap()   
    } 
}
