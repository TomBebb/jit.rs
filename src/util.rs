use libc::*;
use std::fmt::Error;
use std::ffi::CStr;
use std::{mem, ptr, str};
use std::ops::{Deref, Drop};
pub fn oom() -> ! {
    panic!("out of memory")
}

pub fn dump<F>(cb: F) -> Result<String, Error> where F:FnOnce(*mut FILE) {
    unsafe {
        let mut pair = [0, 0];
        if pipe(pair.as_mut_ptr()) == -1 {
            return Err(Error)
        }
        let file = fdopen(pair[1], b"w".as_ptr() as *const c_char);
        if file.is_null() {
            return Err(Error)
        }
        cb(file);
        fclose(file);
        let file = fdopen(pair[0], b"r".as_ptr() as *const c_char);
        if file.is_null() {
            return Err(Error)
        }
        let mut chars:[c_char; 64] = mem::zeroed();
        let mut text = String::new();
        loop {
            let ptr = fgets(chars.as_mut_ptr(), chars.len() as c_int, file);
            let bytes = chars.split(|&c| c == 0).next().unwrap();
            let bytes = mem::transmute(bytes);
            text.push_str(str::from_utf8_unchecked(bytes));
            if ptr.is_null() {
                break
            }
        }
        fclose(file);
        Ok(text)
    }
}
pub fn from_ptr_opt<R, T>(ptr: *mut T) -> Option<R> where R:From<*mut T> {
    if ptr.is_null() {
        None
    } else {
        Some(from_ptr(ptr))
    }
}
pub fn from_ptr<R, T>(ptr: *mut T) -> R where R:From<*mut T> {
    From::from(ptr)
}

pub struct CString {
    ptr: *mut c_char
}
impl CString {
    pub unsafe fn from_ptr(v: *mut c_char) -> CString {
        CString {
            ptr: v
        }
    }
}
impl<'a> From<&'a str> for CString {
    fn from(text: &'a str) -> CString {
        unsafe {
            let bytes = text.as_bytes();
            let new_bytes: *mut c_char = malloc(bytes.len() + 1) as *mut c_char;
            ptr::copy(bytes.as_ptr() as *const c_char, new_bytes, bytes.len());
            ptr::write(new_bytes.offset(bytes.len() as isize), 0);
            CString::from_ptr(new_bytes)
        }
    }
}
impl Deref for CString {
    type Target = CStr;
    fn deref(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.ptr as *const c_char) }
    }
}
impl Drop for CString {
    fn drop(&mut self) {
        unsafe { free(self.ptr as *mut c_void) };
    }
}