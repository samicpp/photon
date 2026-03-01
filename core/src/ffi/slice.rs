use core::slice;
use std::ptr;

#[repr(C)]
#[derive(Debug)]
pub struct FfiSlice{
    pub owned: bool,
    pub len: usize,
    pub cap: usize,
    pub ptr: *const u8,
}

impl FfiSlice{
    pub fn from_string(string: String) -> Self{
        let bytes = string.into_bytes();
        let ptr = bytes.as_ptr();
        let len = bytes.len();
        let cap = bytes.capacity();
        std::mem::forget(bytes);

        Self {
            owned: true,
            len,
            cap,
            ptr,
        }
    }
    pub fn from_vec(vec: Vec<u8>) -> Self{
        let ptr = vec.as_ptr();
        let len = vec.len();
        let cap = vec.capacity();
        std::mem::forget(vec);

        Self {
            owned: true,
            len,
            cap,
            ptr,
        }
    }
    pub fn from_str(str_slice: &str) -> Self{
        let ptr = str_slice.as_ptr();
        let len = str_slice.len();

        Self {
            owned: false,
            len,
            ptr,
            cap: len,
        }
    }
    pub fn from_buf(slice: &[u8]) -> Self{
        let ptr = slice.as_ptr();
        let len = slice.len();

        Self {
            owned: false,
            len,
            ptr,
            cap: len,
        }
    }

    pub const fn empty() -> Self{
        Self { len: 0, cap: 0, ptr: ptr::null(), owned: false }
    }

    pub fn free(self) {
        if self.owned && self.ptr != ptr::null(){
            drop(self.to_vec());
        }
    }
    pub fn to_string(self) -> Option<String>{
        if !self.owned {
            None
        }
        else {
            unsafe { Some(String::from_raw_parts(self.ptr as *mut u8, self.len, self.cap)) }
        }
    }
    pub fn to_vec(self) -> Option<Vec<u8>>{
        if !self.owned { None }
        else{
            unsafe { Some(Vec::from_raw_parts(self.ptr as *mut u8, self.len, self.cap)) }
        }
    }
    pub fn to_owned(self) -> Self {
        if self.owned { self }
        else {
            Self::from_vec(self.as_bytes().to_vec())
        }
    }
    pub fn as_bytes(&self) -> &[u8]{
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
    pub fn as_bytes_mut(&self) -> &mut [u8]{
        unsafe { slice::from_raw_parts_mut(self.ptr as *mut u8, self.len) }
    }
    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        str::from_utf8(self.as_bytes())
    }
    pub fn as_str_lossy(&self) -> std::borrow::Cow<'_, str>{
        String::from_utf8_lossy(self.as_bytes())
    }
}

unsafe impl Sync for FfiSlice{}
unsafe impl Send for FfiSlice{}

impl From<String> for FfiSlice{
    fn from(value: String) -> Self {
        Self::from_string(value)
    }
}
impl From<Vec<u8>> for FfiSlice{
    fn from(value: Vec<u8>) -> Self {
        Self::from_vec(value)
    }
}
impl From<&str> for FfiSlice{
    fn from(value: &str) -> Self {
        Self::from_str(value)
    }
}
impl From<&[u8]> for FfiSlice{
    fn from(value: &[u8]) -> Self {
        Self::from_buf(value)
    }
}
impl From<&String> for FfiSlice{
    fn from(value: &String) -> Self {
        Self::from_str(value)
    }
}
impl From<&Vec<u8>> for FfiSlice{
    fn from(value: &Vec<u8>) -> Self {
        Self::from_buf(value)
    }
}
impl Drop for FfiSlice{
    fn drop(&mut self) {
        unsafe {
            if self.owned {
                drop(Vec::from_raw_parts(self.ptr as *mut u8, self.len, self.cap));
            }
        }
    }
}

pub trait ToFfiSlice {
    fn to_ffi_slice(self) -> FfiSlice;
}
pub trait AsFfiSlice {
    fn as_ffi_slice(&self) -> FfiSlice;
}
impl<I: Into<FfiSlice>> ToFfiSlice for I {
    fn to_ffi_slice(self) -> FfiSlice {
        self.into()
    }
}
impl<I: AsRef<[u8]>> AsFfiSlice for I {
    fn as_ffi_slice(&self) -> FfiSlice {
        self.as_ref().into()
    }
}
