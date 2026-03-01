use tokio::runtime::Runtime;
use crate::ffi::{futures::{self, FfiFuture}, slice::FfiSlice};
use std::{ffi::{CStr, c_void}, ptr, sync::{OnceLock, atomic::Ordering}};


// tokio

pub static RT: OnceLock<Runtime> = OnceLock::new();

#[unsafe(no_mangle)]
pub extern "C" fn init_rt() -> bool{
    if let Ok(rt) = tokio::runtime::Builder::new_multi_thread().enable_all().build(){
        RT.set(rt).is_ok()
    }
    else{
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn has_init() -> bool{
    RT.get().is_some()
}

pub fn spawn_task<F: Future<Output = ()> + Send + 'static>(future: F) {
    RT.get().unwrap().spawn(future);
}




// futures

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_new(cb: Option<extern "C" fn(*mut c_void, *mut c_void)>, userdata: *mut c_void) -> *mut FfiFuture{
    Box::into_raw(FfiFuture::new_boxed(cb, userdata))
}

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_state(fut: *const FfiFuture) -> u8{
    unsafe { (*fut).state.load(Ordering::Acquire) }
}

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_result(fut: *const FfiFuture) -> *mut c_void{
    unsafe {
        if (*fut).state.load(Ordering::Acquire) == futures::READY{
            *(*fut).result.get()
        }
        else {
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_take_result(fut: *const FfiFuture) -> *mut c_void{
    unsafe {
        if (*fut).state.load(Ordering::Acquire) == futures::READY{
            let rptr = (*fut).result.get();
            let result = *rptr;
            *rptr = ptr::null_mut();
            result
        }
        else {
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_cancel(fut: *const FfiFuture) {
    unsafe { (*fut).cancel() }
}

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_complete(fut: *const FfiFuture, result: *mut c_void) {
    unsafe { (*fut).complete(result) }
}

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_free(fut: *mut FfiFuture) {
    unsafe { drop(Box::from_raw(fut)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_await(fut: *mut FfiFuture) {
    unsafe {
        let rfut = (*fut).to_future();
        RT.get().unwrap().block_on(async move {
            rfut.await;
        })
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_get_errno(fut: *mut FfiFuture) -> i32 {
    unsafe {
        *(*fut).errno.get()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_get_errmsg(fut: *mut FfiFuture) -> *const FfiSlice {
    unsafe {
        (*fut).errmsg.get()
    }
}


// slice

#[unsafe(no_mangle)]
pub extern "C" fn free_slice(slice: FfiSlice) {
    slice.free();
}

// test

#[unsafe(no_mangle)]
pub extern "C" fn add_i64(x: i64, y: i64) -> i64 {
    x + y
}

#[unsafe(no_mangle)]
pub extern "C" fn panic_test(message: *const i8) -> ! {
    if message.is_null() {
        panic!("")
    }
    else {
        unsafe {
            let cstr = CStr::from_ptr(message);
            let cstr = cstr.to_string_lossy();
            panic!("{}", cstr);
        }
    }
}
