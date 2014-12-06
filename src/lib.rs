#![feature(unboxed_closures)]

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

pub struct FileDialog {
    title: String,
    dimen: [u32, ..2],
    samples: u8,
    background: Color,
    select: SelectType,
    starting_path: Path,
}

impl FileDialog {
    pub fn with_title<S: StrAllocating>(title: S) -> FileDialog {
        FileDialog {
            title: title.into_string(),
            dimen: [0, ..2],
            samples: 4,
            background: Color::new(0.9, 0.9, 0.9, 1.0), // Should be a nice light-grey
            select: SelectType::File,
        }
    }

    pub fn set_width(mut self, width: u32) -> FileDialog {
        self.dimen[0] = width;
        self   
    }

    pub fn set_height(mut self, height: u32) -> FileDialog {
        self.dimen[1] = height;
        self  
    }

    pub fn set_dimensions(mut self, width: u32, height: u32) -> FileDialog {
        self.dimen = [width, height];
        self    
    }

    pub fn set_samples(mut self, samples: u8) -> FileDialog {
        self.samples = samples;
        self
    }

    pub fn set_background(mut self, background: Color) -> FileDialog {
        self.background = background;
        self  
    }

    pub fn set_select(mut self, select: SelectType) -> FileDialog {
        self.select = select;
        self    
    }

    pub fn set_starting_path(mut self, path: Path) -> 
    
    // How should we format the trait bounds here?
    /// Show the dialog
    pub fn show<W: Window, F: FnOnce(OpenGl, WindowSettings) -> W, WinFn: F + 'static>
    (self, win_fn: WinFn) -> FilePromise {
        let background = self.background;
        let select = self.select;
        let starting_path = self.starting_path;

        let settings = WindowSettings {
            title: self.title,
            size: self.dimen,
            samples: self.samples,
            fullscreen: false,
            exit_on_esc: true    
        };
        
        let (promise, tx) = FilePromise::new();
        
                                    
    }

    fn explode(self) -> (DialogSettings, WindowSettings) {
        (
            DialogSettings {
                background: self.background,    
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

fn render_file_dialog<W: Window>(background: Background, tx: Sender<Path>) {
        
}

pub struct FilePromise {
    opt: Option<Path>,
    rx: Receiver<Path>,
}

impl FilePromise {
    pub fn new() -> (Promise<Path>, Sender<Path>) {
        let (tx, rx) = channel();

        (
            Promise {
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
