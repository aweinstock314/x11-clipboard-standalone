#![feature(libc)]

extern crate libc;
extern crate xlib;

use std::borrow::ToOwned;
use std::ffi::CString;
use std::mem::{size_of, transmute};

use libc::*;
use xlib::*;

pub struct ClipboardContext {
    display: *mut Display,
    window: Window,
    selection: Atom,
}

impl ClipboardContext {
    pub fn new() -> Result<ClipboardContext, &'static str> {
        // http://sourceforge.net/p/xclip/code/HEAD/tree/trunk/xclip.c
        let dpy = unsafe { XOpenDisplay(0 as *mut c_char) };
        if dpy.is_null() {
            return Err("XOpenDisplay")
        }
        let win = unsafe { XCreateSimpleWindow(dpy, XDefaultRootWindow(dpy), 0, 0, 1, 1, 0, 0, 0) };
        if win == 0 {
            return Err("XCreateSimpleWindow")
        }
        if unsafe { XSelectInput(dpy, win, PropertyChangeMask.bits()) } == 0 {
            return Err("XSelectInput");
        }
        let sel = unsafe { XmuInternAtom(dpy, _XA_CLIPBOARD) };
        if sel == 0 {
            return Err("XA_CLIPBOARD")
        }
        Ok(ClipboardContext {
            display: dpy,
            window: win,
            selection: sel,
        })
    }
    pub fn get_contents(&self) -> String {
        enum XCOutState {
            None,
            SentConvSel,
            BadTarget,
            Incr,
        };
        fn xcout(dpy: *mut Display, win: Window, evt: &mut Vec<u8>,
                sel: Atom, target: Atom, type_: &mut Atom, dest: &mut Vec<u8>,
                context: &mut XCOutState) {
            let pty_cstr = CString::new("SERVO_CLIPBOARD_OUT").unwrap(); // TODO: lazy_static! (possibly?)
            let pty_atom = unsafe { XInternAtom(dpy, transmute::<*const c_char, *mut c_char>(pty_cstr.as_ptr()), 0) };
            let incr_cstr = CString::new("INCR").unwrap();
            let incr_atom = unsafe { XInternAtom(dpy, transmute::<*const c_char, *mut c_char>(incr_cstr.as_ptr()), 0) };
            match *context {
                XCOutState::None => {
                    unsafe { XConvertSelection(dpy, sel, target, pty_atom, win, 0); } // CurrentTime = 0 = special flag (TODO: rust-xlib)
                    *context = XCOutState::SentConvSel;
                    return;
                },
                XCOutState::SentConvSel => {
                    let event: &mut XSelectionEvent = unsafe { transmute(evt.as_mut_ptr()) };
                    if event._type != SelectionNotify {
                        return;
                    }
                    if event.property == 0 {
                        *context = XCOutState::BadTarget;
                        return;
                    }
                    println!("{:?}\n{:?}\n", evt, event._type);
                    let mut buffer: *mut c_uchar = std::ptr::null_mut();
                    let mut pty_format: c_int = 0;
                    let mut pty_size: c_ulong = 0;
                    let mut pty_items: c_ulong = 0;
                    unsafe {
                        XGetWindowProperty(dpy, win, pty_atom, 0, 0, 0, 0,
                                            transmute::<&mut Atom, *mut Atom>(type_), 
                                            transmute::<&mut c_int, *mut c_int>(&mut pty_format),
                                            transmute::<&mut c_ulong, *mut c_ulong>(&mut pty_items),
                                            transmute::<&mut c_ulong, *mut c_ulong>(&mut pty_size),
                                            transmute::<&mut *mut c_uchar, *mut *mut c_uchar>(&mut buffer));
                        //XFree(transmute::<*mut c_uchar, *mut c_void>(buffer));
                    }
                    println!("{:?} {:?} {:?} {:?}", buffer, pty_format, pty_size, pty_items);
                    if *type_ == incr_atom {
                        unsafe {
                            XDeleteProperty(dpy, win, pty_atom);
                            XFlush(dpy);
                        }
                        *context = XCOutState::Incr;
                        return;
                    }
                    unsafe {
                        XGetWindowProperty(dpy, win, pty_atom, 0, pty_size as c_long, 0, 0,
                                            transmute::<&mut Atom, *mut Atom>(type_), 
                                            transmute::<&mut c_int, *mut c_int>(&mut pty_format),
                                            transmute::<&mut c_ulong, *mut c_ulong>(&mut pty_items),
                                            transmute::<&mut c_ulong, *mut c_ulong>(&mut pty_size),
                                            transmute::<&mut *mut c_uchar, *mut *mut c_uchar>(&mut buffer));
                    }
                    fn mach_itemsize(format: c_int) -> usize {
                        match format {
                            8 => size_of::<c_char>(),
                            16 => size_of::<c_short>(),
                            32 => size_of::<c_long>(),
                            _ => 0, // TODO: should this panic! instead?
                        }
                    }
                    let pty_machsize: c_ulong = pty_items * (mach_itemsize(pty_format) as c_ulong);
                    dest.push_all(unsafe { std::slice::from_raw_parts_mut(buffer, (pty_machsize as usize) / size_of::<u8>()) });
                },
                XCOutState::BadTarget => (),
                XCOutState::Incr => (),
            }
        }
        let mut sel_buf = vec![];
        let mut sel_type = 0;
        let mut state = XCOutState::None;
        let mut event: Vec<u8> = vec![0; get_size_for_XEvent()];
        let mut target = XA_STRING; // TODO: XA_UTF8_STRING(dpy)
        loop {
            println!("1");
            if let XCOutState::None = state {} else {
                println!("2");
                unsafe { XNextEvent(self.display, event.as_mut_ptr() as *mut XEvent) };
            }
            println!("3");
            xcout(self.display, self.window, &mut event, self.selection, target, &mut sel_type, &mut sel_buf, &mut state);
        }
        String::from_utf8(sel_buf).unwrap()
    }
}

impl Drop for ClipboardContext {
    fn drop(&mut self) {
        println!("display is {:?}", self.display);
        let retcode = unsafe { XCloseDisplay(self.display) };
        if retcode == 0 {
            panic!("XCloseDisplay failed. (return code {})", retcode);
        }
    }
}

fn main() {
    let clipboard_ctx = ClipboardContext::new().unwrap();
    println!("{}", clipboard_ctx.get_contents());
    println!("{:?}", _XA_CLIPBOARD);
    println!("{:?}", size_of::<ClipboardContext>());
    println!("{:?}", get_size_for_XEvent());
}
