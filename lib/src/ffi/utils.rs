use core::ffi::c_void;
use std::{os::fd::{FromRawFd, RawFd}, ptr};

use httprs_core::ffi::{futures::FfiFuture, slice::{AsFfiSlice, FfiSlice}};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

use crate::{DynStream, errno::TYPE_ERR, spawn_task_with};


pub fn heap_ptr<T>(thing: T) -> *mut T{
    Box::into_raw(Box::new(thing))
}
pub fn heap_void_ptr<T>(thing: T) -> *mut c_void {
    Box::into_raw(Box::new(thing)) as *mut c_void
}
pub fn heap_const_ptr<T>(thing: T) -> *const T{
    Box::into_raw(Box::new(thing))
}


#[repr(C)]
#[derive(Debug)]
pub struct FfiDuoStream {
    pub one: *mut DynStream, // idk
    pub two: *mut DynStream, // 
}


#[unsafe(no_mangle)]
pub extern "C" fn create_duplex(bufsize: usize) -> FfiDuoStream {
    let duo = tokio::io::duplex(bufsize);
    FfiDuoStream {
        one: heap_ptr(duo.0.into()),
        two: heap_ptr(duo.1.into()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tcp_from_fd(fd: RawFd) -> *mut DynStream {
    unsafe {
        let tcp = std::net::TcpStream::from_raw_fd(fd);
        
        if let Ok(tcp) = TcpStream::from_std(tcp) {
            heap_ptr(tcp.into())
        } 
        else { 
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tcp_peek(fut: *mut FfiFuture, ffi: *mut DynStream, buf: *mut FfiSlice){
    unsafe {
        let ffi = &*ffi;
        let fut = &*fut;
        let buf = (*buf).as_bytes_mut();

        if let DynStream::Tcp(tcp) = ffi {
            spawn_task_with(fut, async move {
                Ok(heap_void_ptr(tcp.peek(buf).await))
            });
        }
        else{
            fut.cancel_with_err(TYPE_ERR, "socket not tcp".into())
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tls_get_alpn(stream: *mut DynStream) -> FfiSlice {
    unsafe {
        match &*stream {
            DynStream::TcpTls(tls) => {
                let (_, info) = tls.get_ref();
                info.alpn_protocol().map(|alpn| alpn.to_vec().as_ffi_slice()).unwrap_or(FfiSlice::empty())
            }
            DynStream::TlsDuplex(tls) => {
                let (_, info) = tls.get_ref();
                info.alpn_protocol().map(|alpn| alpn.to_vec().as_ffi_slice()).unwrap_or(FfiSlice::empty())
            }
            _ => FfiSlice::empty(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stream_read(fut: *mut FfiFuture, stream: *mut DynStream, buf: *mut FfiSlice){
    unsafe {
        let stream = &mut *stream;
        let fut = &*fut;
        let buf = (*buf).as_bytes_mut();

        spawn_task_with(fut, async move {
            Ok(heap_void_ptr(stream.read(buf).await))
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stream_write(fut: *mut FfiFuture, stream: *mut DynStream, buf: *mut FfiSlice){
    unsafe {
        let stream = &mut *stream;
        let fut = &*fut;
        let buf = (*buf).as_bytes();

        spawn_task_with(fut, async move {
            Ok(heap_void_ptr(stream.write(buf).await))
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn stream_write_all(fut: *mut FfiFuture, stream: *mut DynStream, buf: *mut FfiSlice){
    unsafe {
        let stream = &mut *stream;
        let fut = &*fut;
        let buf = (*buf).as_bytes_mut();

        spawn_task_with(fut, async move {
            stream.write_all(buf).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stream_flush(fut: *mut FfiFuture, stream: *mut DynStream){
    unsafe {
        let stream = &mut *stream;
        let fut = &*fut;

        spawn_task_with(fut, async move {
            stream.flush().await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stream_shutdown(fut: *mut FfiFuture, stream: *mut DynStream){
    unsafe {
        let stream = &mut *stream;
        let fut = &*fut;

        spawn_task_with(fut, async move {
            stream.shutdown().await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stream_free(stream: *mut DynStream){
    unsafe {
        drop(Box::from_raw(stream))
    }
}

