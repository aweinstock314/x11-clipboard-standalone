extern crate xlib;
extern crate libc;

use std::borrow::ToOwned;
use xlib::{Display, Window};
use xlib::{XOpenDisplay, XCloseDisplay};
use xlib::{XCreateSimpleWindow, XDefaultRootWindow};
use libc::*;

pub struct ClipboardContext {
    display: *mut Display,
    window: Window,
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
        Ok(ClipboardContext {
            display: dpy,
            window: win,
        })
    }
    pub fn get_contents(&self) -> String {
        "dummy string".to_owned()
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
}
