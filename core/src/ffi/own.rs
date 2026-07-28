use crate::{futures::{self, FfiFuture}, runtime::{RT, TokioSettings}, slice::FfiSlice};
use std::{ffi::{CStr, c_char, c_void}, ptr, sync::atomic::Ordering};


// tokio
#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_builder(multi_threaded: bool) -> *mut TokioSettings { Box::into_raw(Box::new(TokioSettings::new_unset(multi_threaded))) }

#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_set_worker_threads(tok: *mut TokioSettings, worker_threads: usize) { unsafe { (*tok).worker_threads = Some(worker_threads); } }
#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_unset_worker_threads(tok: *mut TokioSettings) { unsafe { (*tok).worker_threads = None; } }

#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_set_thread_name(tok: *mut TokioSettings, thread_name: *const c_char) { unsafe { (*tok).thread_name = Some(CStr::from_ptr(thread_name).to_string_lossy().to_string()); } }
#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_unset_thread_name(tok: *mut TokioSettings) { unsafe { (*tok).thread_name = None; } }

#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_set_event_interval(tok: *mut TokioSettings, event_interval: u32) { unsafe { (*tok).event_interval = Some(event_interval); } }
#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_unset_event_interval(tok: *mut TokioSettings) { unsafe { (*tok).event_interval = None; } }

#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_set_max_io_events_per_tick(tok: *mut TokioSettings, max_io_events_per_tick: usize) { unsafe { (*tok).max_io_events_per_tick = Some(max_io_events_per_tick); } }
#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_unset_max_io_events_per_tick(tok: *mut TokioSettings) { unsafe { (*tok).max_io_events_per_tick = None; } }

#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_set_global_queue_interval(tok: *mut TokioSettings, global_queue_interval: u32) { unsafe { (*tok).global_queue_interval = Some(global_queue_interval); } }
#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_unset_global_queue_interval(tok: *mut TokioSettings) { unsafe { (*tok).global_queue_interval = None; } }

#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_set_thread_keep_alive_ns(tok: *mut TokioSettings, thread_keep_alive_ns: u64) { unsafe { (*tok).thread_keep_alive_ns = Some(thread_keep_alive_ns); } }
#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_unset_thread_keep_alive_ns(tok: *mut TokioSettings) { unsafe { (*tok).thread_keep_alive_ns = None; } }

#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_set_thread_stack_size(tok: *mut TokioSettings, thread_stack_size: usize) { unsafe { (*tok).thread_stack_size = Some(thread_stack_size); } }
#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_unset_thread_stack_size(tok: *mut TokioSettings) { unsafe { (*tok).thread_stack_size = None; } }

#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_set_max_blocking_threads(tok: *mut TokioSettings, max_blocking_threads: usize) { unsafe { (*tok).max_blocking_threads = Some(max_blocking_threads); } }
#[unsafe(no_mangle)] pub extern "C" fn tokio_rt_unset_max_blocking_threads(tok: *mut TokioSettings) { unsafe { (*tok).max_blocking_threads = None; } }



#[unsafe(no_mangle)]
pub extern "C" fn init_rt() -> bool{
    if let Ok(rt) = tokio::runtime::Builder::new_multi_thread().enable_all().build(){
        RT.set(rt);
        true
    }
    else{
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn init_rt_with_settings(tok: *mut TokioSettings) -> bool{
    let tok = unsafe { Box::from_raw(tok) };
    if tok.multi_threaded {
        let mut rt = tokio::runtime::Builder::new_multi_thread();
        
        rt.enable_all();
        if let Some(t) = tok.worker_threads { rt.worker_threads(t); }
        if let Some(n) = tok.thread_name { rt.thread_name(n); }
        if let Some(e) = tok.event_interval { rt.event_interval(e); }
        if let Some(m) = tok.max_io_events_per_tick { rt.max_io_events_per_tick(m); }
        if let Some(g) = tok.global_queue_interval { rt.global_queue_interval(g); }
        if let Some(d) = tok.thread_keep_alive_ns { rt.thread_keep_alive(std::time::Duration::from_nanos(d)); }
        if let Some(s) = tok.thread_stack_size { rt.thread_stack_size(s); }
        if let Some(b) = tok.max_blocking_threads { rt.max_blocking_threads(b); }

        match rt.build() {
            Ok(rt) => {
                RT.set(rt);
                true
            },
            Err(_) => {
                false
            }
        }
    }
    else {
        let mut rt = tokio::runtime::Builder::new_current_thread();
        
        rt.enable_all();
        if let Some(n) = tok.thread_name { rt.thread_name(n); }
        if let Some(s) = tok.thread_stack_size { rt.thread_stack_size(s); }

        match rt.build() {
            Ok(rt) => {
                RT.set(rt);
                true
            },
            Err(_) => {
                false
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn has_init() -> bool{
    RT.isset()
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
pub extern "C" fn ffi_future_cancel_with_err(fut: *const FfiFuture, code: i32, msg: FfiSlice) {
    unsafe { (*fut).cancel_with_err(code, msg) }
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
        let rfut = &mut *fut;
        RT.block_on(async move {
            let _ = rfut.await;
        });
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

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_reset(fut: *mut FfiFuture) {
    unsafe {
        (*fut) = FfiFuture::default()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_get_userdata(fut: *const FfiFuture) -> *mut c_void{
    unsafe {
        *(*fut).userdata.get()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn ffi_future_set_userdata(fut: *const FfiFuture, userdata: *mut c_void) {
    unsafe {
        *(*fut).userdata.get() = userdata;
    }
}


// async_ffi(crate) FfiFuture

#[unsafe(no_mangle)]
pub extern "C" fn rt_spawn_async_ffi_future(fut: async_ffi::FfiFuture<()>) {
    RT.spawn(fut);
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
pub extern "C" fn panic_test(message: *const c_char) {
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
