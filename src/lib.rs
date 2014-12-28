#![feature(unboxed_closures, macro_rules, globs)]

extern crate shader_version;
extern crate event;
extern crate event_loop;
extern crate conrod;
extern crate graphics;
extern crate window;
extern crate opengl_graphics;
extern crate current;
extern crate sdl2_window;

use conrod::*;

use current::Set;
use event::Event;
use event_loop::{Events, Ups, MaxFps};
use opengl_graphics::Gl;
use opengl_graphics::glyph_cache::GlyphCache as Font;
use shader_version::opengl::OpenGL;
use sdl2_window::Sdl2Window;
use window::WindowSettings;

use std::borrow::ToOwned;
use std::default::Default;
use std::io::fs::{mod, PathExtensions};
use std::io::IoResult;
use std::thread::{Thread, JoinGuard};
use std::os;

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

impl FileDialog {
    pub fn new<'a, S: IntoCow<'a, String, str>>(title: S, font: Font) -> FileDialog {
        FileDialog {
            title: title.into_cow().into_owned(),
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

    pub fn set_starting_path(mut self, starting_path: Path) -> FileDialog {
        self.starting_path = starting_path;
        self
    }
    
    pub fn set_filter_hidden(mut self, filter_hidden: bool) -> FileDialog {
        self.filter_hidden = filter_hidden;
        self
    }
    
    // How should we format the trait bounds here?
    /// Show the dialog
    pub fn show(self, gl: OpenGL) -> JoinGuard<Option<Path>> {
        let dialog = DialogSettings {
            background: self.background,
            select: self.select,
            starting_path: self.starting_path,
            font: self.font,
            filter_hidden: self.filter_hidden,
        };

        let window =  WindowSettings {
            title: self.title,
            size: self.dimen,
            samples: self.samples,
            fullscreen: false,
            exit_on_esc: true,
        };         

        Thread::spawn(move || render_file_dialog(dialog, window, gl))       
    }
}

/// An enum describing the file selection behavior.
#[deriving(PartialEq, Eq)]
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
    filter_hidden: bool,
}

impl DialogSettings {
    fn into_state(self) -> (DialogState, Font) {
        (
            DialogState {
                dir: self.starting_path,
                selected: None,
                result: None,
                exit: false,
                paths: Vec::new(),
                background: self.background,
                select: self.select,
                dir_changed: true,
                pages: 0,
                cur_page: 0,
                filter_hidden: self.filter_hidden,
            },
            self.font,
        )
    }    
}

fn render_file_dialog(dialog: DialogSettings, window: WindowSettings, gl: OpenGL) -> Option<Path> {
    let (mut state, font) = dialog.into_state();
    state.update_paths();

    let window = Sdl2Window::new(gl, window);
    let mut event_loop = Events::new(window).set(Ups(120)).set(MaxFps(60));
    let mut gl = Gl::new(gl);
    
    let ref mut uic = UiContext::new(font, Theme::default());
    let ref mut buf: Buffers = Default::default();

    if let SelectType::SaveFile(ref mut opt_file) = state.select {
        opt_file.take().map(|s: String| buf.filename = s);    
    }

    for event in event_loop {
        if state.exit { break; }

        uic.handle_event(&event);
        match event {
            Event::Render(args) => {
                gl.draw([0, 0, args.width as i32, args.height as i32], |_, gl| {
                    draw_dialog_ui(gl, uic, &mut state, buf);
                });
            },
            _ => {}    
        }
    }

    state.result
}

/// Like format! except writes to an existing string.
macro_rules! write_str(
    ($s:expr, $fmt:expr, $($arg:expr),+) => (
        {
            let vec = unsafe { $s.as_mut_vec() };
            // Should always be `Ok(())` unless something went wrong.
            (write!(vec, $fmt, $($arg),+)).unwrap();
        }    
    )
);

#[deriving(Default)]
struct Buffers {
    dir: String,
    page: String,
    filename: String,
    selected: String,
}

impl Buffers {
    fn set_page(&mut self, page: uint, total: uint) {
        self.page.clear();
        write_str!(self.page, "Page: {} Total: {}", page, total);    
    }

    fn set_selected(&mut self, selected: &Path) {
        self.selected.clear();
        write_str!(self.selected, "Selected: {}", selected.filename_display());
    }

    fn set_dir(&mut self, dir: &Path) {
        self.dir.clear();
        write_str!(self.dir, "{}", dir.display());
    }
}

struct DialogState {
    dir: Path,
    selected: Option<uint>,
    result: Option<Path>,
    exit: bool,
    paths: Vec<Path>,
    background: Color,
    select: SelectType,
    dir_changed: bool,
    pages: uint,
    cur_page: uint,
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
        self.pages = count / PER_PAGE;
        self.cur_page = 1;

        if count % PER_PAGE != 0 { self.pages += 1; }
    }

    fn up_dir(&mut self) {
        self.dir_changed = self.dir.pop();
        self.update_paths();
    }

    fn select(&mut self, num: uint, buf: &mut Buffers) {
        // Double-clicked
        if self.selected == Some(num) {
            let path = self.paths.remove(num).unwrap();
            
            if self.select.show_files() && !path.is_dir() {
                self.result = Some(path);
                self.exit = true;   
            } else {
                self.update_dir(path);
            }            
        } else {
            self.selected = Some(num);
            buf.set_selected(&self.paths[num]);
        }        
    }

    fn save(&mut self, filename: &str) {
        self.result = Some(self.dir.join(filename));
        self.exit = true;    
    }

    fn next_page(&mut self, buf: &mut Buffers) {
        if self.cur_page < self.pages { 
            self.cur_page += 1;
            buf.set_page(self.cur_page, self.pages);
        } 
    }

    fn prev_page(&mut self, buf: &mut Buffers) {
        if self.cur_page > 1 { 
            self.cur_page -= 1;
            buf.set_page(self.cur_page, self.pages);
        }    
    }
}

const COLS: uint = 5;
const ROWS: uint = 8;
const PER_PAGE: uint = COLS * ROWS;
const CHAR_LIMIT: uint = 24;

fn draw_dialog_ui(gl: &mut Gl, uic: &mut UiContext, state: &mut DialogState, buf: &mut Buffers) {
    uic.background().color(state.background).draw(gl);
 
    if state.dir_changed {
        buf.set_dir(&state.dir);
        buf.set_page(state.cur_page, state.pages);
        state.dir_changed = false;
    }
                  
    const UP_DIR: u64 = 78;
    
    uic.button(UP_DIR)
        .position(605.0, 5.0)
        .dimensions(30.0, 30.0)
        .label("Up")
        .callback(|| state.up_dir())
        .draw(gl);

    uic.label(&*buf.dir)
        .position(5.0, 5.0)
        .size(24)
        .draw(gl);
 
    const PREV_PAGE: u64 = 199;
        
    uic.button(PREV_PAGE)
        .position(5.0, 445.0)
        .dimensions(90.0, 30.0)
        .label("Previous")
        .callback(|| state.prev_page(buf))
        .draw(gl);

    const NEXT_PAGE: u64 = PREV_PAGE + 1;

    uic.button(NEXT_PAGE)
        .right_from(PREV_PAGE, 125.0)
        .dimensions(90.0, 30.0)
        .label("Next")
        .callback(|| state.next_page(buf))
        .draw(gl);

    uic.label(&*buf.page)
        .right_from(PREV_PAGE, 15.0)
        .size(24)
        .draw(gl);   
        
    const FILE_START_ID: u64 = 365;
    uic.widget_matrix(COLS, ROWS)
        .position(5.0, 35.0)
        .dimensions(600.0, 400.0)
        .cell_padding(5.0, 5.0)
        .each_widget(|uic, _, x, y, pt, dimen| {
            use std::cmp;
            let idx = (y * COLS * state.cur_page) + x;
    
            if idx >= state.paths.len() { return; }

            let label = state.paths[idx].filename_str().unwrap().to_owned();

            let button = uic.button(FILE_START_ID + idx as u64)
                .point(pt)
                .dimensions(dimen[0], dimen[1]) 
                .label(label.slice_to(cmp::min(label.len(), CHAR_LIMIT))).label_font_size(18);

                if state.selected == Some(idx) {
                    button.color(Color::new(0.5, 0.9, 0.5, 1.0))
                } else { button }
                .callback(|| state.select(idx, buf))
                .draw(gl);                
        });
        
    const CANCEL: u64 = 205;
    const CONFIRM: u64 = 206;

    uic.button(CANCEL)
        .right_from(NEXT_PAGE, 140.0)
        .dimensions(90.0, 30.0)
        .callback(|| state.exit = true)
        .label("Cancel")
        .draw(gl);

    {
        let confirm = uic.button(CONFIRM)
            .right_from(CANCEL, 5.0)
            .dimensions(90.0, 30.0);

        if let Some(idx) = state.selected {
            confirm.label(match state.select {
                    SelectType::SaveFile(_) if !state.paths[idx].is_dir() => "Save",
                    _ => "Open",
                })
                .callback(|| state.select(idx, buf))
                .draw(gl);                
        } else if state.select == SelectType::Folder {
            confirm.label("Select Folder")
                .callback(|| {
                    state.result = Some(state.dir.clone()); 
                    state.exit = true
                })
                .draw(gl);
        } else if let SelectType::SaveFile(_) = state.select {
            let confirm = confirm.label("Save");

            if !buf.filename.is_empty() {
                confirm.callback(|| state.save(&*buf.filename))
            } else {
                confirm
            }.draw(gl);
        } else {
            confirm.label("Open").draw(gl);
        }
    }

    const FILENAME: u64 = 210;
    if let SelectType::SaveFile(_) = state.select {
        uic.text_box(FILENAME, &mut buf.filename)
            .right_from(NEXT_PAGE, 5.0)
            .dimensions(130.0, 30.0)
            .draw(gl);
    } else if let Some(_) = state.selected {
        uic.label(&*buf.selected)
            .right_from(NEXT_PAGE, 5.0)
            .size(18)
            .draw(gl);  
    }
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

