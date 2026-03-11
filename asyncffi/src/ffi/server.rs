use std::{ffi::CStr, net::SocketAddr, os::fd::{FromRawFd, RawFd}, ptr};

use http::{http1::server::Http1Socket, shared::{HttpClient, HttpMethod, HttpSocket, HttpType, HttpVersion}};
use httprs_core::ffi::{futures::FfiFuture, slice::FfiSlice, own::spawn_task};
use tokio::{io::AsyncWriteExt, net::TcpListener};

use crate::{DynStream, errno::{Errno, TYPE_ERR}, ffi::utils::{heap_ptr, heap_void_ptr}, servers::{DynHttpSocket, detect_prot}, spawn_task_with};


#[repr(C)]
#[derive(Debug)]
pub struct FfiBundle{
    pub sock: *mut DynStream,
    pub addr: *const SocketAddr,
}


#[repr(C)]
#[derive(Debug)]
pub struct FfiClient{
    pub owned: bool,
    pub valid: bool,

    pub head_complete: bool,
    pub body_complete: bool,
    
    pub path: FfiSlice,
    pub method: u8,
    pub version: u8,
    pub method_str: FfiSlice,

    pub headers_len: usize,
    pub headers_cap: usize,
    pub headers: *const FfiHeaderPair,
    pub body: FfiSlice,

    pub host: FfiSlice,
    pub scheme: FfiSlice,
}

#[repr(C)]
#[derive(Debug)]
pub struct FfiHeaderPair{
    pub nam: FfiSlice,
    pub val: FfiSlice,
}

impl FfiClient{
    pub fn from_owned(client: HttpClient) -> Self{
        let mut pairs = Vec::new();
        client.headers.into_iter().for_each(|(h,vs)|vs.into_iter().for_each(|v| pairs.push(FfiHeaderPair { nam: FfiSlice::from_string(h.clone()), val: FfiSlice::from_string(v) })));
        let pair_ptr = pairs.as_ptr();
        let pairs_len = pairs.len();
        let pairs_cap = pairs.capacity();
        std::mem::forget(pairs);

        Self {
            owned: true,
            valid: client.valid,

            head_complete: client.head_complete,
            body_complete: client.body_complete,

            path: FfiSlice::from_string(client.path),
            method: match client.method { HttpMethod::Unknown(_) => 0, HttpMethod::Get => 1, HttpMethod::Head => 2, HttpMethod::Post => 3, HttpMethod::Put => 4, HttpMethod::Delete => 5, HttpMethod::Connect => 6, HttpMethod::Options => 7, HttpMethod::Trace => 8 },
            version: match client.version { HttpVersion::Unknown(_) => 0, HttpVersion::Debug => 1, HttpVersion::Http09 => 2, HttpVersion::Http10 => 3, HttpVersion::Http11 => 4, HttpVersion::Http2 => 5, HttpVersion::Http3 => 6 },
            method_str: FfiSlice::from_string(client.method.to_string()),

            headers_len: pairs_len,
            headers_cap: pairs_cap,
            headers: pair_ptr,
            body: FfiSlice::from_vec(client.body),

            host: client.host.and_then(|h|Some(FfiSlice::from_string(h))).unwrap_or(FfiSlice::empty()),
            scheme: client.scheme.and_then(|s|Some(FfiSlice::from_string(s))).unwrap_or(FfiSlice::empty()),
        }
    }
    pub fn from(client: &HttpClient) -> Self{
        let mut pairs = Vec::new();
        client.headers.iter().for_each(|(h,vs)|vs.into_iter().for_each(|v| pairs.push(FfiHeaderPair { nam: FfiSlice::from_str(h), val: FfiSlice::from_str(v) })));
        let pair_ptr = pairs.as_ptr();
        let pairs_len = pairs.len();
        let pairs_cap = pairs.capacity();
        std::mem::forget(pairs);

        Self {
            owned: false,
            valid: client.valid,

            head_complete: client.head_complete,
            body_complete: client.body_complete,

            path: FfiSlice::from_str(&client.path),
            method: match client.method { HttpMethod::Unknown(_) => 0, HttpMethod::Get => 1, HttpMethod::Head => 2, HttpMethod::Post => 3, HttpMethod::Put => 4, HttpMethod::Delete => 5, HttpMethod::Connect => 6, HttpMethod::Options => 7, HttpMethod::Trace => 8 },
            version: match client.version { HttpVersion::Unknown(_) => 0, HttpVersion::Debug => 1, HttpVersion::Http09 => 2, HttpVersion::Http10 => 3, HttpVersion::Http11 => 4, HttpVersion::Http2 => 5, HttpVersion::Http3 => 6 },
            method_str: FfiSlice::from_string(client.method.to_string()),

            headers_len: pairs_len,
            headers_cap: pairs_cap,
            headers: pair_ptr,
            body: FfiSlice::from_buf(&client.body),

            host: client.host.as_ref().and_then(|h|Some(FfiSlice::from_str(h))).unwrap_or(FfiSlice::empty()),
            scheme: client.scheme.as_ref().and_then(|s|Some(FfiSlice::from_str(s))).unwrap_or(FfiSlice::empty()),
        }
    }

    pub fn free(self){
        self.method_str.free();
        let pairs = unsafe { Vec::from_raw_parts(self.headers as *mut FfiHeaderPair, self.headers_len, self.headers_cap) };
        
        if self.owned{
            self.path.free();
            self.body.free();
            self.host.free();
            self.scheme.free();


            for h in pairs {
                h.nam.free();
                h.val.free();
            }
        }
    }
}


#[unsafe(no_mangle)]
pub extern "C" fn tcp_server_new(fut: *mut FfiFuture, string: *mut i8){
    unsafe {
        let addr = CStr::from_ptr(string).to_string_lossy().to_string();
        let fut = &*fut;

        spawn_task_with(fut, async move {
            let lis = TcpListener::bind(addr).await?;
            Ok(heap_void_ptr(lis))
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn tcp_server_from_fd(fd: RawFd) -> *mut TcpListener {
    unsafe {
        let tcp = std::net::TcpListener::from_raw_fd(fd);
        
        if let Ok(tcp) = TcpListener::from_std(tcp) {
            heap_ptr(tcp)
        } 
        else { 
            ptr::null_mut()
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn tcp_server_free(listener: *mut TcpListener) {
    unsafe {
        drop(Box::from_raw(listener))
    }
}

// #[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn tcp_server_accept(fut: *mut FfiFuture, server: *mut TcpListener){
    unsafe {
        let server = &mut *server;
        let fut = &*fut;

        spawn_task_with(fut, async move {
            let (sock, addr) = server.accept().await?;
            let sock = sock.into();
            let sock = heap_ptr(sock);

            let addr = heap_ptr(addr);

            let ffi = FfiBundle {
                sock,
                addr,
            };

            Ok(heap_void_ptr(ffi))
        });
    }
}
// #[allow(improper_ctypes_definitions)]
// #[unsafe(no_mangle)]
/*pub extern "C" fn server_loop(fut: *mut FfiFuture, server: *mut FfiServer, cb: extern "C" fn(*mut FfiBundle)){
    unsafe {
        let mut ser = Box::from_raw(server);
        let fut = Box::from_raw(fut);

        spawn_task(async move {
            loop {
                match ser.boxed.accept().await{
                    Ok((addr, sock)) => cb(Box::into_raw(Box::new(FfiBundle { sock, addr }))),
                    Err(e) => {
                        fut.cancel_with_err(e.get_errno(), e.to_string().into());
                        break;
                    },
                }
            }

            let _ = Box::into_raw(ser);
            let _ = Box::into_raw(fut);
        });
    }
}*/

#[unsafe(no_mangle)]
pub extern "C" fn addr_is_ipv4(addr: *const SocketAddr) -> bool{
    unsafe{
        (*addr).is_ipv4()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn addr_is_ipv6(addr: *const SocketAddr) -> bool{
    unsafe{
        (*addr).is_ipv6()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn get_addr_str(addr: *const SocketAddr) -> FfiSlice{
    unsafe{
        FfiSlice::from_string((*addr).to_string())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tcp_detect_prot(fut: *mut FfiFuture, stream: *mut DynStream){
    unsafe {
        let stream = &mut *stream;
        let fut = &*fut;

        spawn_task(async move {
            if let DynStream::Tcp(tcp) = stream {
                fut.complete(heap_void_ptr(detect_prot(tcp).await))
            }
            else {
                fut.cancel_with_err(TYPE_ERR, "socket not tcp".into())
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http1_new(ffi: *mut DynStream, bufsize: usize) -> *mut DynHttpSocket{
    unsafe{
        let ffi = *Box::from_raw(ffi);
        let http = Http1Socket::new(ffi, bufsize);
        let dhtt = DynHttpSocket::Http1(http);
        heap_ptr(dhtt)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_get_type(http: *mut DynHttpSocket) -> u8{
    unsafe {
        match (*http).get_type() {
            HttpType::Http1 => 1,
            HttpType::Http2 => 2,
            HttpType::Http3 => 3,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_read_client(fut: *mut FfiFuture, http: *mut DynHttpSocket){
    unsafe{
        let http = &mut *http;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            http.read_client().await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_read_until_complete(fut: *mut FfiFuture, http: *mut DynHttpSocket){
    unsafe{
        let http = &mut *http;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            http.read_until_complete().await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_read_until_head_complete(fut: *mut FfiFuture, http: *mut DynHttpSocket){
    unsafe{
        let http = &mut *http;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            http.read_until_head_complete().await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_set_header(http: *mut DynHttpSocket, pair: FfiHeaderPair){
    unsafe{
        let name = pair.nam.as_str_lossy();
        let value = pair.val.as_str_lossy();

        (*http).set_header(&name, &value);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_add_header(http: *mut DynHttpSocket, pair: FfiHeaderPair){
    unsafe{
        let name = pair.nam.as_str_lossy();
        let value = pair.val.as_str_lossy();

        (*http).add_header(&name, &value);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_del_header(http: *mut DynHttpSocket, name: FfiSlice){
    unsafe{
        let name = name.as_str_lossy();
        let _ = (*http).del_header(&name);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_write(fut: *mut FfiFuture, http: *mut DynHttpSocket, buf: FfiSlice){
    unsafe{
        let http = &mut *http;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            http.write(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_close(fut: *mut FfiFuture, http: *mut DynHttpSocket, buf: FfiSlice){
    unsafe{
        let http = &mut *http;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            http.close(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_flush(fut: *mut FfiFuture, http: *mut DynHttpSocket){
    unsafe{
        let fut = &*fut;
        let http = &mut *http;
        spawn_task_with(fut, async move{
            http.flush().await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_get_fficlient(http: *mut DynHttpSocket) -> *mut FfiClient {
    unsafe{
        heap_ptr(FfiClient::from(&(*http).get_client()))
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_free_fficlient(client: *mut FfiClient) {
    unsafe { 
        let cl = Box::from_raw(client);
        cl.free();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_client_get_method(http: *mut DynHttpSocket) -> u8 {
    unsafe {
        match (*http).get_client().method { HttpMethod::Unknown(_) => 0, HttpMethod::Get => 1, HttpMethod::Head => 2, HttpMethod::Post => 3, HttpMethod::Put => 4, HttpMethod::Delete => 5, HttpMethod::Connect => 6, HttpMethod::Options => 7, HttpMethod::Trace => 8 }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_client_get_method_str(http: *mut DynHttpSocket) -> FfiSlice {
    unsafe {
        (&(*http).get_client().method.to_string()).into()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_client_get_path(http: *mut DynHttpSocket) -> FfiSlice {
    unsafe {
        (&(*http).get_client().path).into()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_client_get_version(http: *mut DynHttpSocket) -> u8 {
    unsafe {
        match (*http).get_client().version { HttpVersion::Unknown(_) => 0, HttpVersion::Debug => 1, HttpVersion::Http09 => 2, HttpVersion::Http10 => 3, HttpVersion::Http11 => 4, HttpVersion::Http2 => 5, HttpVersion::Http3 => 6 }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_client_has_header(http: *mut DynHttpSocket, name: FfiSlice) -> bool {
    unsafe{
        (*http).get_client().headers.contains_key(name.as_str_lossy().as_ref())
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_client_has_header_count(http: *mut DynHttpSocket, name: FfiSlice) -> usize {
    unsafe{
        (*http).get_client().headers.get(name.as_str_lossy().as_ref()).and_then(|h|Some(h.len())).unwrap_or(0)
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_client_get_first_header(http: *mut DynHttpSocket, name: FfiSlice) -> FfiSlice {
    unsafe{
        (*http).get_client().headers.get(name.as_str_lossy().as_ref()).and_then(|h|Some(FfiSlice::from_string(h[0].clone()))).unwrap_or(FfiSlice::empty())
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_client_get_header(http: *mut DynHttpSocket, name: FfiSlice, index: usize) -> FfiSlice {
    unsafe{
        (*http).get_client().headers.get(name.as_str_lossy().as_ref()).and_then(
            |h|h.get(index)
            .and_then(|h|Some(FfiSlice::from_string(h.clone())))
        ).unwrap_or(FfiSlice::empty())
    }
}
// #[unsafe(no_mangle)]
/*pub extern "C" fn http_client_get_all_headers(http: *mut DynHttpSocket) -> FfiSlice {
    unsafe{
        let mut pairs = Vec::new();
        (*http).get_client().headers.iter().for_each(|(h,vs)|vs.into_iter().for_each(|v| pairs.push(FfiHeaderPair { nam: FfiSlice::from_str(h), val: FfiSlice::from_str(v) })));
        let pair_ptr = pairs.as_ptr();
        let pairs_len = pairs.len();
        let pairs_cap = pairs.capacity();
        std::mem::forget(pairs);
    }
}*/
#[unsafe(no_mangle)]
pub extern "C" fn http_client_get_body(http: *mut DynHttpSocket) -> FfiSlice {
    unsafe {
        (&(*http).get_client().body).into()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_free(http: *mut DynHttpSocket){
    unsafe{
        drop(Box::from_raw(http));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http1_direct_write(fut: *mut FfiFuture, http: *mut DynHttpSocket, buf: FfiSlice){
    unsafe{
        let http = &mut *http;
        let fut = &*fut;
        spawn_task(async move{
            match http {
                DynHttpSocket::Http1(one) => {
                    match one.netw.write_all(buf.as_bytes()).await {
                        Ok(_) => fut.complete(ptr::null_mut()),
                        Err(e) => fut.cancel_with_err(e.get_errno(), e.to_string().into()),
                    }
                }
                _ => fut.cancel_with_err(TYPE_ERR, "not http1".into()),
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http1_websocket(fut: *mut FfiFuture, http: *mut DynHttpSocket){
    unsafe{
        let http = *Box::from_raw(http);
        let fut = &*fut;
        
        match http {
            DynHttpSocket::Http1(one) => {
                spawn_task_with(fut, async move {
                    let ws = one.websocket().await?;
                    Ok(heap_void_ptr(ws))
                })
            }
            _ => fut.cancel_with_err(TYPE_ERR, "not http1".into()),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http1_h2c(fut: *mut FfiFuture, http: *mut DynHttpSocket){
    unsafe{
        let http = *Box::from_raw(http);
        let fut = &*fut;
        
        match http {
            DynHttpSocket::Http1(one) => {
                spawn_task_with(fut, async move {
                    let h2 = one.h2c(None).await?;
                    Ok(heap_void_ptr(h2))
                })
            }
            _ => fut.cancel_with_err(TYPE_ERR, "not http1".into()),
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http1_h2_prior_knowledge(fut: *mut FfiFuture, http: *mut DynHttpSocket){
    unsafe{
        let http = *Box::from_raw(http);
        let fut = &*fut;

        match http {
            DynHttpSocket::Http1(one) => {
                spawn_task_with(fut, async move {
                    let h2 = one.http2_prior_knowledge().await?;
                    Ok(heap_void_ptr(h2))
                })
            }
            _ => fut.cancel_with_err(TYPE_ERR, "not http1".into()),
        }
    }
}
