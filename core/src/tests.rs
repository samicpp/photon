#![allow(unused_imports)]
use std::sync::atomic::Ordering;
#[cfg(test)]

use std::{sync::Arc, thread, time::Duration, ptr};
use crate::futures::{self, FfiFuture};

#[test]
fn one_is_one(){
    assert!(1 == 1);
}


#[test]
fn ffi_future_sleep(){
    let fut: FfiFuture<()> = FfiFuture::new(None, ptr::null_mut());

    assert!(fut.state.load(Ordering::Acquire) == futures::PENDING);
    fut.complete(ptr::null_mut());
    assert!(fut.state.load(Ordering::Acquire) == futures::READY);
    unsafe { assert!(*fut.result.get() == ptr::null_mut()) };

}

