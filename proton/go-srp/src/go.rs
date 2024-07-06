#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(deref_nullptr)]
#![allow(clippy::all)]
#![allow(dead_code)]
#![allow(improper_ctypes)]

use std::ffi::{CStr, CString};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::os::raw::{c_char, c_uchar, c_void};
use std::ptr::NonNull;

include!(concat!(env!("OUT_DIR"), "/go-srp.rs"));

#[doc(hidden)]
pub(crate) struct SafeGoString<'a> {
    str: CString,
    size: isize,
    p: PhantomData<&'a str>,
}

impl<'a> SafeGoString<'a> {
    pub(crate) fn new(value: &'a str) -> Self {
        Self {
            str: CString::new(value).unwrap(),
            size: value.len() as isize,
            p: PhantomData,
        }
    }

    pub(crate) unsafe fn as_go_string(&self) -> GoString {
        GoString {
            p: self.str.as_ptr(),
            n: self.size,
        }
    }
}

#[doc(hidden)]
pub struct CBytes {
    ptr: NonNull<c_uchar>,
    len: usize,
}

impl CBytes {
    pub unsafe fn new(ptr: *mut c_uchar, len: usize) -> Self {
        Self {
            ptr: NonNull::new(ptr).unwrap(),
            len,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr() as *mut u8, self.len) }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl AsRef<[u8]> for CBytes {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr() as *mut u8, self.len) }
    }
}

impl Drop for CBytes {
    fn drop(&mut self) {
        unsafe {
            CGoFree(self.ptr.as_ptr() as *mut c_void);
        }
    }
}

#[doc(hidden)]
pub struct OwnedCStr {
    cstr: NonNull<c_char>,
}

impl OwnedCStr {
    pub unsafe fn new(str: *mut c_char) -> Self {
        Self {
            cstr: NonNull::new(str).unwrap(),
        }
    }
}

impl OwnedCStr {
    pub fn to_string(&self) -> String {
        unsafe {
            CStr::from_ptr(self.cstr.as_ptr())
                .to_str()
                .unwrap()
                .to_string()
        }
    }

    pub fn to_cstring(&self) -> CString {
        unsafe { CString::from_raw(self.cstr.as_ptr()) }
    }
}

impl AsRef<[u8]> for OwnedCStr {
    fn as_ref(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.cstr.as_ptr()).to_bytes() }
    }
}

impl Drop for OwnedCStr {
    fn drop(&mut self) {
        unsafe {
            CGoFree(self.cstr.as_ptr() as *mut c_void);
        }
    }
}

impl Debug for OwnedCStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(unsafe { CStr::from_ptr(self.cstr.as_ptr()) }, f)
    }
}
