use core::slice;
use std::{ptr, sync::Arc};

use http::{http2::{client::Http2Request, core::{Http2Frame, Http2Settings}, server::Http2Socket, session::{Http2Session, Mode}}};
use httprs_core::ffi::{futures::FfiFuture, slice::{FfiSlice, ToFfiSlice}};
use tokio::io::{BufReader, ReadHalf, WriteHalf};

use crate::{DynStream, clients::DynHttpRequest, ffi::{server::FfiHeaderPair, utils::{heap_ptr, heap_void_ptr}}, servers::DynHttpSocket, spawn_task_with};

pub type DynH2Sess = Http2Session<BufReader<ReadHalf<DynStream>>, WriteHalf<DynStream>>;



#[unsafe(no_mangle)]
pub extern "C" fn http2_new(stream: *mut DynStream, bufsize: usize) -> *const DynH2Sess {
    unsafe {
        let stream = *Box::from_raw(stream);
        let (netr, netw) = tokio::io::split(stream);
        let netr = BufReader::with_capacity(bufsize, netr);
        let h2 = Http2Session::with(netr, netw, Mode::Ambiguous, true, Http2Settings::default());
        let h2 = Arc::into_raw(Arc::new(h2));
        h2
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_new_client(stream: *mut DynStream, bufsize: usize) -> *const DynH2Sess {
    unsafe {
        let stream = *Box::from_raw(stream);
        let (netr, netw) = tokio::io::split(stream);
        let netr = BufReader::with_capacity(bufsize, netr);
        let h2 = Http2Session::with(netr, netw, Mode::Client, true, Http2Settings::default());
        let h2 = Arc::into_raw(Arc::new(h2));
        h2
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_new_server(stream: *mut DynStream, bufsize: usize) -> *const DynH2Sess {
    unsafe {
        let stream = *Box::from_raw(stream);
        let (netr, netw) = tokio::io::split(stream);
        let netr = BufReader::with_capacity(bufsize, netr);
        let h2 = Http2Session::with(netr, netw, Mode::Server, true, Http2Settings::default());
        let h2 = Arc::into_raw(Arc::new(h2));
        h2
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_with(stream: *mut DynStream, bufsize: usize, mode: u8, strict: bool, settings: FfiSlice) -> *const DynH2Sess {
    unsafe {
        let stream = *Box::from_raw(stream);
        let (netr, netw) = tokio::io::split(stream);
        let netr = BufReader::with_capacity(bufsize, netr);

        let mode = match mode {
            1 => Mode::Client,
            2 => Mode::Server,
            _ => Mode::Ambiguous,
        };

        let settings = Http2Settings::from(settings.as_bytes());

        let h2 = Http2Session::with(netr, netw, mode, strict, settings);
        let h2 = Arc::into_raw(Arc::new(h2));
        h2
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_free(session: *const DynH2Sess) {
    unsafe {
        drop(Arc::from_raw(session));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http2_read_preface(fut: *const FfiFuture, session: *const DynH2Sess) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            Ok(heap_void_ptr(sess.read_preface().await?))
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_send_preface(fut: *const FfiFuture, session: *const DynH2Sess) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            sess.send_preface().await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_next(fut: *const FfiFuture, session: *const DynH2Sess) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            if let Some(open) = sess.next().await? {
                Ok(heap_void_ptr(open))
            }
            else {
                Ok(ptr::null_mut())
            }
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_read_raw(fut: *const FfiFuture, session: *const DynH2Sess) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            let frame = sess.read_frame().await?;
            Ok(heap_void_ptr(frame.source.to_ffi_slice()))
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_handle_raw(fut: *const FfiFuture, session: *const DynH2Sess, frame: FfiSlice) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;
        let frame = Http2Frame::from_owned(if frame.owned { frame.to_vec().unwrap() } else { frame.as_bytes().to_vec() });
        let frame = if let Some(frame) = frame { frame } else { return };

        spawn_task_with(fut, async move{
            if let Some(open) = sess.handle(frame).await? {
                Ok(heap_void_ptr(open))
            }
            else {
                Ok(ptr::null_mut())
            }
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_open_stream(session: *const DynH2Sess) -> u32 {
    unsafe {
        (*session).open_stream().unwrap_or(0)
    }
}


#[unsafe(no_mangle)]
pub extern "C" fn http2_send_data(fut: *const FfiFuture, session: *const DynH2Sess, stream_id: u32, end: bool, buf: FfiSlice) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            sess.send_data(stream_id, end, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_send_headers(fut: *const FfiFuture, session: *const DynH2Sess, stream_id: u32, end: bool, headers: *const FfiHeaderPair, length: usize) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;
        let mut head = Vec::with_capacity(length);

        for hv in slice::from_raw_parts(headers, length) {
            head.push((hv.nam.as_bytes(), hv.val.as_bytes()));
        }

        spawn_task_with(fut, async move{
            sess.send_headers(stream_id, end, &head).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http2_send_priority(fut: *const FfiFuture, session: *const DynH2Sess, stream_id: u32, dependency: u32, weight: u8) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            sess.send_priority(stream_id, dependency, weight).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http2_send_rst_stream(fut: *const FfiFuture, session: *const DynH2Sess, stream_id: u32, code: u32) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            sess.send_rst_stream(stream_id, code).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http2_send_settings(fut: *const FfiFuture, session: *const DynH2Sess, settings: FfiSlice) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;
        let settings = Http2Settings::from(settings.as_bytes());

        spawn_task_with(fut, async move{
            sess.send_settings(settings).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_send_settings_default(fut: *const FfiFuture, session: *const DynH2Sess) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;
        let settings = Http2Settings::default();

        spawn_task_with(fut, async move{
            sess.send_settings(settings).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_send_settings_default_no_push(fut: *const FfiFuture, session: *const DynH2Sess) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;
        let settings = Http2Settings::default_no_push();

        spawn_task_with(fut, async move{
            sess.send_settings(settings).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_send_settings_maximum(fut: *const FfiFuture, session: *const DynH2Sess) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;
        let settings = Http2Settings::maximum();

        spawn_task_with(fut, async move{
            sess.send_settings(settings).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http2_send_push_promise(fut: *const FfiFuture, session: *const DynH2Sess, associate_id: u32, promise_id: u32, headers: *const FfiHeaderPair, length: usize) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;
        let mut head = Vec::with_capacity(length);

        for hv in slice::from_raw_parts(headers, length) {
            head.push((hv.nam.as_bytes(), hv.val.as_bytes()));
        }

        spawn_task_with(fut, async move{
            sess.send_push_promise(associate_id, promise_id, &head).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http2_send_ping(fut: *const FfiFuture, session: *const DynH2Sess, ack: bool, buf: FfiSlice) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            sess.send_ping(ack, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http2_send_goaway(fut: *const FfiFuture, session: *const DynH2Sess, stream_id: u32, code: u32, buf: FfiSlice) {
    unsafe {
        let sess = &*session;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            sess.send_goaway(stream_id, code, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}


// Arc::increment_strong_count(conf);
// Arc::from_raw(conf)

#[unsafe(no_mangle)]
pub extern "C" fn http2_client_handler(session: *const DynH2Sess, stream_id: u32) -> *mut DynHttpRequest {
    unsafe {
        let session = {
            Arc::increment_strong_count(session);
            Arc::from_raw(session)
        };

        if let Ok(req) = Http2Request::new(stream_id, session) {
            let req = DynHttpRequest::Http2(req);
            heap_ptr(req)
        }
        else {
            ptr::null_mut()
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http2_server_handler(session: *const DynH2Sess, stream_id: u32) -> *mut DynHttpSocket {
    unsafe {
        let session = {
            Arc::increment_strong_count(session);
            Arc::from_raw(session)
        };
        
        if let Ok(req) = Http2Socket::new(stream_id, session) {
            let req = DynHttpSocket::Http2(req);
            heap_ptr(req)
        }
        else {
            ptr::null_mut()
        }
    }
}
