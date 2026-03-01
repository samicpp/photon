use std::{cell::UnsafeCell, ffi::c_void, future::poll_fn, ptr, sync::{atomic::{AtomicU8, Ordering}}, task::{Poll, Waker}};

use crate::ffi::slice::FfiSlice;

pub const PENDING: u8 = 0;
pub const READY: u8 = 1;
pub const CANCELED: u8 = 2;

#[derive(Debug)]
pub struct FfiFuture{
    pub state: AtomicU8,
    pub result: UnsafeCell<*mut c_void>,
    pub userdata: UnsafeCell<*mut c_void>,
    pub callback: Option<extern "C" fn(*mut c_void, *mut c_void)>,
    pub waker: UnsafeCell<Option<Waker>>,

    pub errno: UnsafeCell<i32>,
    pub errmsg: UnsafeCell<FfiSlice>,
}

impl FfiFuture{
    pub fn new(cb: Option<extern "C" fn(*mut c_void, *mut c_void)>, userdata: *mut c_void) -> Self{
        FfiFuture { 
            state: AtomicU8::new(PENDING), 
            result: UnsafeCell::new(ptr::null_mut()), 
            callback: cb, 
            userdata: UnsafeCell::new(userdata),
            waker: UnsafeCell::new(None), 
            errno: UnsafeCell::new(-1),
            errmsg: UnsafeCell::new(FfiSlice::empty()),
        }
    }
    pub fn new_boxed(cb: Option<extern "C" fn(*mut c_void, *mut c_void)>, userdata: *mut c_void) -> Box<Self>{
        Box::new(Self::new(cb, userdata))
    }

    pub fn cancel(&self){
        self.state.swap(CANCELED, Ordering::AcqRel);

        if let Some(cb) = &self.callback{
            unsafe { cb(*self.userdata.get(), *self.result.get()); }
        }

        unsafe{
            if let Some(w) = (*self.waker.get()).take(){
                w.wake();
            }
        }
    }
    pub fn cancel_with_err(&self, code: i32, msg: FfiSlice){
        self.state.swap(CANCELED, Ordering::AcqRel);

        unsafe{
            (*self.errno.get()) = code;
            (*self.errmsg.get()) = msg;
        }

        if let Some(cb) = &self.callback{
            unsafe { cb(*self.userdata.get(), *self.result.get()); }
        }

        unsafe{
            if let Some(w) = (*self.waker.get()).take(){
                w.wake();
            }
        }
    }

    pub fn complete(&self, result: *mut c_void){
        if self.state.swap(READY, Ordering::AcqRel) != PENDING {
            return;
        }
        
        unsafe {
            (*self.result.get()) = result;
        }

        if let Some(cb) = &self.callback{
            unsafe { cb(*self.userdata.get(), *self.result.get()); }
        }

        unsafe{
            if let Some(w) = (*self.waker.get()).take(){
                w.wake();
            }
        }
    }

    pub fn to_future(&self) -> impl Future<Output = *mut c_void> + '_{
        poll_fn(move |cx|{
            match self.state.load(Ordering::Acquire){
                READY => unsafe { Poll::Ready(*self.result.get()) },
                CANCELED => Poll::Ready(ptr::null_mut()),
                _ => {
                    unsafe {
                        let wakptr = &mut *self.waker.get();
                        if wakptr.is_none() { 
                            *wakptr = Some(cx.waker().clone());
                        }
                    }
                    Poll::Pending
                }
            }
        })
    }
}

unsafe impl Sync for FfiFuture {}
unsafe impl Send for FfiFuture {}
