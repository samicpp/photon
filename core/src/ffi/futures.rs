use std::{cell::UnsafeCell, ffi::c_void, ptr, sync::{atomic::{AtomicU8, Ordering}}, task::{Poll, Waker}};

use crate::ffi::slice::FfiSlice;

pub const PENDING: u8 = 0;
pub const READY: u8 = 1;
pub const CANCELED: u8 = 2;

#[derive(Debug)]
pub struct FfiFuture<T = c_void>{
    pub state: AtomicU8,
    pub result: UnsafeCell<*mut T>,
    pub userdata: UnsafeCell<*mut c_void>,
    pub callback: Option<extern "C" fn(*mut c_void, *mut T)>,
    pub waker: UnsafeCell<Option<Waker>>,

    pub errno: UnsafeCell<i32>,
    pub errmsg: UnsafeCell<FfiSlice>,
}

impl<T> FfiFuture<T>{
    pub const fn default() -> Self {
        FfiFuture { 
            state: AtomicU8::new(PENDING), 
            result: UnsafeCell::new(ptr::null_mut()), 
            callback: None, 
            userdata: UnsafeCell::new(ptr::null_mut()),
            waker: UnsafeCell::new(None), 
            errno: UnsafeCell::new(-1),
            errmsg: UnsafeCell::new(FfiSlice::empty()),
        }
    }
    pub const fn new(cb: Option<extern "C" fn(*mut c_void, *mut T)>, userdata: *mut c_void) -> Self{
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
    pub fn new_boxed(cb: Option<extern "C" fn(*mut c_void, *mut T)>, userdata: *mut c_void) -> Box<Self>{
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

    pub fn complete(&self, result: *mut T){
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

    // pub fn to_future(&self) -> impl Future<Output = *mut c_void> + '_{
    //     poll_fn(move |cx|{
    //         match self.state.load(Ordering::Acquire){
    //             READY => unsafe { Poll::Ready(*self.result.get()) },
    //             CANCELED => Poll::Ready(ptr::null_mut()),
    //             _ => {
    //                 unsafe {
    //                     let wakptr = &mut *self.waker.get();
    //                     if wakptr.is_none() { 
    //                         *wakptr = Some(cx.waker().clone());
    //                     }
    //                 }
    //                 Poll::Pending
    //             }
    //         }
    //     })
    // }
}

impl<T> Future for FfiFuture<T> {
    type Output = Result<*mut T, i32>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        unsafe {
            let fut = self.get_mut();
            match fut.state.load(Ordering::Acquire){
                READY => Poll::Ready(Ok(*fut.result.get())),
                CANCELED => Poll::Ready(Err(*fut.errno.get())),
                _ => {
                    *fut.waker.get() = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }
}

unsafe impl Sync for FfiFuture {}
unsafe impl Send for FfiFuture {}
