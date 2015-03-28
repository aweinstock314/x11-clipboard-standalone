#![feature(collections, core, libc)]

extern crate libc;
extern crate xlib;

use std::mem::{size_of, transmute};

use libc::*;
use xlib::*;

pub struct ClipboardContext {
    display: *mut Display,
    window: Window,
    selection: Atom,
    utf8string: Atom,
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
        let utf8 = unsafe { XmuInternAtom(dpy, _XA_UTF8_STRING) };
        if utf8 == 0 {
            return Err("XA_UTF8_STRING")
        }
        Ok(ClipboardContext {
            display: dpy,
            window: win,
            selection: sel,
            utf8string: utf8,
        })
    }
    pub fn get_contents(&self) -> Result<String, &str> {
        enum XCOutState {
            None,
            SentConvSel,
            BadTarget,
            Incr,
        };
        fn mach_itemsize(format: c_int) -> usize {
            match format {
                8 => size_of::<c_char>(),
                16 => size_of::<c_short>(),
                32 => size_of::<c_long>(),
                _ => panic!("unexpected format for mach_itemsize: {}", format),
            }
        }
        fn xcout(dpy: *mut Display, win: Window, evt: &mut Vec<u8>,
                sel: Atom, target: Atom, type_: &mut Atom, dest: &mut Vec<u8>,
                context: &mut XCOutState) {
            let pty_atom = unsafe { XInternAtom(dpy, b"SERVO_CLIPBOARD_OUT\0".as_ptr() as *mut i8, 0) };
            let incr_atom = unsafe { XInternAtom(dpy, b"INCR\0".as_ptr() as *mut i8, 0) };

            let mut buffer: *mut c_uchar = std::ptr::null_mut();
            let mut pty_format: c_int = 0;
            let mut pty_size: c_ulong = 0;
            let mut pty_items: c_ulong = 0;

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
                    unsafe {
                        XGetWindowProperty(dpy, win, pty_atom, 0, 0, 0, 0,
                                            transmute::<&mut Atom, *mut Atom>(type_), 
                                            transmute::<&mut c_int, *mut c_int>(&mut pty_format),
                                            transmute::<&mut c_ulong, *mut c_ulong>(&mut pty_items),
                                            transmute::<&mut c_ulong, *mut c_ulong>(&mut pty_size),
                                            transmute::<&mut *mut c_uchar, *mut *mut c_uchar>(&mut buffer));
                        // This XFree is present in xclib.c, but rustc doesn't like it, and I'm not quite sure why.
                        /*
                        src/main.rs:85:31: 85:77 error: mismatched types:
                         expected `*mut libc::types::common::c95::c_void`,
                            found `*mut libc::types::common::c95::c_void`
                        (expected enum `libc::types::common::c95::c_void`,
                            found a different enum `libc::types::common::c95::c_void`) [E0308]
                        src/main.rs:85                         XFree(transmute::<*mut c_uchar, *mut c_void>(buffer));
                        */
                        //XFree(transmute::<*mut c_uchar, *mut c_void>(buffer));
                    }
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
                    let pty_machsize: c_ulong = pty_items * (mach_itemsize(pty_format) as c_ulong);
                    dest.push_all(unsafe { std::slice::from_raw_parts_mut(buffer, (pty_machsize as usize) / size_of::<u8>()) });
                    *context = XCOutState::None;
                },
                XCOutState::BadTarget => panic!("should be unreachable"),
                XCOutState::Incr => {
                    let event: &mut XPropertyEvent = unsafe { transmute(evt.as_mut_ptr()) };
                    if event._type != PropertyNotify {
                        return;
                    }
                    if event.state != 0 { // 0 == PropertyNewValue
                        return;
                    }
                    unsafe {
                        XGetWindowProperty(dpy, win, pty_atom, 0, 0, 0, 0,
                                            transmute::<&mut Atom, *mut Atom>(type_), 
                                            transmute::<&mut c_int, *mut c_int>(&mut pty_format),
                                            transmute::<&mut c_ulong, *mut c_ulong>(&mut pty_items),
                                            transmute::<&mut c_ulong, *mut c_ulong>(&mut pty_size),
                                            transmute::<&mut *mut c_uchar, *mut *mut c_uchar>(&mut buffer));
                    }
                    //XFree(transmute::<*mut c_uchar, *mut c_void>(buffer));
                    if pty_size == 0 {
                        unsafe {
                            XDeleteProperty(dpy, win, pty_atom);
                            XFlush(dpy);
                        }
                        *context = XCOutState::None;
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
                    let pty_machsize: c_ulong = pty_items * (mach_itemsize(pty_format) as c_ulong);
                    dest.push_all(unsafe { std::slice::from_raw_parts_mut(buffer, (pty_machsize as usize) / size_of::<u8>()) });
                    *context = XCOutState::None;
                },
            }
        }
        let mut sel_buf = vec![];
        let mut sel_type = 0;
        let mut state = XCOutState::None;
        let mut event: Vec<u8> = vec![0; get_size_for_XEvent()];
        let mut target = self.utf8string;
        loop {
            if let XCOutState::None = state {} else {
                unsafe { XNextEvent(self.display, event.as_mut_ptr() as *mut XEvent) };
            }
            xcout(self.display, self.window, &mut event, self.selection, target, &mut sel_type, &mut sel_buf, &mut state);
            if let XCOutState::BadTarget = state {
                if target == self.utf8string {
                    state = XCOutState::None;
                    target = XA_STRING;
                    continue;
                }
                else {
                    return Err("unable to negotiate format");
                }
            }
            if let XCOutState::None = state {
                break;
            }
        }
        Ok(String::from_utf8_lossy(sel_buf.as_slice()).into_owned())
    }
}

impl Drop for ClipboardContext {
    fn drop(&mut self) {
        let retcode = unsafe { XCloseDisplay(self.display) };
        if retcode != 0 {
            panic!("XCloseDisplay failed. (return code {})", retcode);
        }
    }
}

fn main() {
    let clipboard_ctx = ClipboardContext::new().unwrap();
    println!("{}", clipboard_ctx.get_contents().unwrap());
}
