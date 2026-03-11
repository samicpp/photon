use std::{ffi::CStr, ptr};

use http::{http1::client::Http1Request, shared::{HttpMethod, HttpRequest, HttpResponse, HttpType}};
use httprs_core::ffi::{futures::FfiFuture, slice::FfiSlice};

use crate::{DynStream, clients::{DynHttpRequest, tcp_connect as ntcpconn, tls_upgrade, tls_upgrade_no_verification}, errno::TYPE_ERR, ffi::{const_enums::methods, server::FfiHeaderPair, utils::{heap_ptr, heap_void_ptr}}, spawn_task_with};

#[repr(C)]
#[derive(Debug)]
pub struct FfiResponse{
    pub owned: bool,
    pub valid: bool,

    pub head_complete: bool,
    pub body_complete: bool,

    pub code: u16,
    pub status: FfiSlice,

    pub headers_len: usize,
    pub headers_cap: usize,
    pub headers: *const FfiHeaderPair,
    pub body: FfiSlice,
}
impl FfiResponse{
    pub fn from(response: &HttpResponse) -> Self {
        let mut pairs = Vec::new();
        response.headers.iter().for_each(|(h,vs)|vs.into_iter().for_each(|v| pairs.push(FfiHeaderPair { nam: FfiSlice::from_str(h), val: FfiSlice::from_str(v) })));
        let pair_ptr = pairs.as_ptr();
        let pairs_len = pairs.len();
        let pairs_cap = pairs.capacity();
        std::mem::forget(pairs);

        Self { 
            owned: false,
            valid: response.valid,
            head_complete: response.head_complete,
            body_complete: response.body_complete,
            code: response.code,
            status: response.status.as_str().into(),
            headers_len: pairs_len,
            headers_cap: pairs_cap,
            headers: pair_ptr,
            body: FfiSlice::from_buf(&response.body),
        }
    }
    pub fn from_owned(response: HttpResponse) -> Self {
        let mut pairs = Vec::new();
        response.headers.into_iter().for_each(|(h,vs)|vs.into_iter().for_each(|v| pairs.push(FfiHeaderPair { nam: FfiSlice::from_string(h.clone()), val: FfiSlice::from_string(v) })));
        let pair_ptr = pairs.as_ptr();
        let pairs_len = pairs.len();
        let pairs_cap = pairs.capacity();
        std::mem::forget(pairs);

        Self { 
            owned: true,
            valid: response.valid,
            head_complete: response.head_complete,
            body_complete: response.body_complete,
            code: response.code,
            status: response.status.into(),
            headers_len: pairs_len,
            headers_cap: pairs_cap,
            headers: pair_ptr,
            body: response.body.into(),
        }
    }
    
    pub fn free(self){
        let pairs = unsafe { Vec::from_raw_parts(self.headers as *mut FfiHeaderPair, self.headers_len, self.headers_cap) };
        
        if self.owned{
            self.status.free();
            self.body.free();

            for h in pairs {
                h.nam.free();
                h.val.free();
            }
        }
    }
}


#[unsafe(no_mangle)]
pub extern "C" fn tcp_connect(fut: *mut FfiFuture, addr: *mut i8){
    unsafe{
        let addr = CStr::from_ptr(addr).to_string_lossy().to_string();
        let fut = &*fut;

        spawn_task_with(fut, async move {
            let tcp = ntcpconn(addr).await?;
            let ptr = heap_void_ptr(DynStream::from(tcp));
            Ok(ptr)
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tcp_tls_connect(fut: *mut FfiFuture, addr: *mut i8, domain: *mut i8, alpns: *mut i8){
    unsafe{
        let addr = CStr::from_ptr(addr).to_string_lossy().to_string();
        let domain = CStr::from_ptr(domain).to_string_lossy().to_string();
        let alpns = CStr::from_ptr(alpns).to_string_lossy().to_string();
        let alpns = alpns.split(',').map(|s|s.as_bytes().to_vec()).collect();
        let fut = &*fut;

        spawn_task_with(fut, async move {
            let tcp = ntcpconn(addr).await?;
            let tls = tls_upgrade(tcp, domain, alpns).await?;
            let ptr = heap_void_ptr(DynStream::from(tls));
            Ok(ptr)
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn tcp_tls_connect_unverified(fut: *mut FfiFuture, addr: *mut i8, domain: *mut i8, alpns: *mut i8){
    unsafe{
        let addr = CStr::from_ptr(addr).to_string_lossy().to_string();
        let domain = CStr::from_ptr(domain).to_string_lossy().to_string();
        let alpns = CStr::from_ptr(alpns).to_string_lossy().to_string();
        let alpns = alpns.split(',').map(|s|s.as_bytes().to_vec()).collect();
        let fut = &*fut;

        spawn_task_with(fut, async move {
            let tcp = ntcpconn(addr).await?;
            let tls = tls_upgrade_no_verification(tcp, domain, alpns).await?;
            let ptr = heap_void_ptr(DynStream::from(tls));
            Ok(ptr)
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http1_request_new(stream: *mut DynStream, bufsize: usize) -> *mut DynHttpRequest{
    unsafe{
        let stream = *Box::from_raw(stream);
        let dreq = Http1Request::new(stream, bufsize).into();
        heap_ptr(dreq)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_req_get_type(http: *mut DynHttpRequest) -> u8{
    unsafe {
        match (*http).get_type() {
            HttpType::Http1 => 1,
            HttpType::Http2 => 2,
            HttpType::Http3 => 3,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_req_set_header(req: *mut DynHttpRequest, pair: FfiHeaderPair){
    unsafe{
        let name = pair.nam.as_str_lossy();
        let value = pair.val.as_str_lossy();

        (*req).set_header(&name, &value);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_req_add_header(req: *mut DynHttpRequest, pair: FfiHeaderPair){
    unsafe{
        let name = pair.nam.as_str_lossy();
        let value = pair.val.as_str_lossy();

        (*req).add_header(&name, &value);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_req_del_header(req: *mut DynHttpRequest, name: FfiSlice){
    unsafe{
        let name = name.as_str_lossy();
        let _ = (*req).del_header(&name);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_req_set_method_str(req: *mut DynHttpRequest, method: FfiSlice){
    unsafe{
        let meth = method.as_str_lossy().as_ref().into();
        (*req).set_method(meth);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_req_set_method_byte(req: *mut DynHttpRequest, method: u8){
    unsafe{
        let meth = match method{
            methods::GET => HttpMethod::Get,
            methods::HEAD => HttpMethod::Head,
            methods::POST => HttpMethod::Post,
            methods::PUT => HttpMethod::Put,
            methods::DELETE => HttpMethod::Delete,
            methods::CONNECT => HttpMethod::Connect,
            methods::OPTIONS => HttpMethod::Options,
            methods::TRACE => HttpMethod::Trace,
            _ => HttpMethod::Unknown(None),
        };
        (*req).set_method(meth);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_req_set_path(req: *mut DynHttpRequest, path: FfiSlice){
    unsafe{
        let path = path.as_str_lossy().to_string();
        (*req).set_path(path);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_req_write(fut: *mut FfiFuture, req: *mut DynHttpRequest, buf: FfiSlice){
    unsafe{
        let req = &mut *req;
        let fut = &*fut;
        spawn_task_with(fut, async move{
            req.write(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_req_send(fut: *mut FfiFuture, req: *mut DynHttpRequest, buf: FfiSlice){
    unsafe{
        let req = &mut *req;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            req.send(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_req_flush(fut: *mut FfiFuture, req: *mut DynHttpRequest){
    unsafe{
        let fut = &*fut;
        let req = &mut *req;
        spawn_task_with(fut, async move{
            req.flush().await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_req_read(fut: *mut FfiFuture, req: *mut DynHttpRequest){
    unsafe{
        let req = &mut *req;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            req.read_response().await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_req_read_until_complete(fut: *mut FfiFuture, req: *mut DynHttpRequest){
    unsafe{
        let req = &mut *req;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            req.read_until_complete().await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_req_read_until_head_complete(fut: *mut FfiFuture, req: *mut DynHttpRequest){
    unsafe{
        let req = &mut *req;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            req.read_until_head_complete().await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_response_get_status_code(req: *mut DynHttpRequest) -> u16 {
    unsafe {
        (*req).get_response().code
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_response_get_status_msg(req: *mut DynHttpRequest) -> FfiSlice {
    unsafe {
        (&(*req).get_response().status).into()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_response_has_header(req: *mut DynHttpRequest, name: FfiSlice) -> bool {
    unsafe{
        (*req).get_response().headers.contains_key(name.as_str_lossy().as_ref())
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_response_has_header_count(req: *mut DynHttpRequest, name: FfiSlice) -> usize {
    unsafe{
        (*req).get_response().headers.get(name.as_str_lossy().as_ref()).and_then(|h|Some(h.len())).unwrap_or(0)
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_response_get_first_header(req: *mut DynHttpRequest, name: FfiSlice) -> FfiSlice {
    unsafe{
        (*req).get_response().headers.get(name.as_str_lossy().as_ref()).and_then(|h|Some(FfiSlice::from_string(h[0].clone()))).unwrap_or(FfiSlice::empty())
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_response_get_header(req: *mut DynHttpRequest, name: FfiSlice, index: usize) -> FfiSlice {
    unsafe{
        (*req).get_response().headers.get(name.as_str_lossy().as_ref()).and_then(
            |h|h.get(index)
            .and_then(|h|Some(FfiSlice::from_string(h.clone())))
        ).unwrap_or(FfiSlice::empty())
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_response_get_body(req: *mut DynHttpRequest) -> FfiSlice {
    unsafe {
        (&(*req).get_response().body).into()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_req_get_ffires(req: *mut DynHttpRequest) -> *const FfiResponse {
    unsafe {
        heap_ptr(FfiResponse::from(&(*req).get_response()))
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http_req_free_ffires(res: *mut FfiResponse) {
    unsafe {
        drop(Box::from_raw(res))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http_req_free(req: *mut DynHttpRequest){
    unsafe{
        drop(Box::from_raw(req));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http1_websocket_strict(fut: *mut FfiFuture, http: *mut DynHttpRequest){
    unsafe{
        let http = *Box::from_raw(http);
        let fut = &*fut;
        
        match http {
            DynHttpRequest::Http1(one) => {
                spawn_task_with(fut, async move {
                    let ws = one.websocket_strict().await?;
                    Ok(heap_void_ptr(ws))
                })
            }
            _ => fut.cancel_with_err(TYPE_ERR, "not http1".into()),
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn http1_websocket_lazy(fut: *mut FfiFuture, http: *mut DynHttpRequest){
    unsafe{
        let http = *Box::from_raw(http);
        let fut = &*fut;

        match http {
            DynHttpRequest::Http1(one) => {
                spawn_task_with(fut, async move {
                    let ws = one.websocket_lazy().await?;
                    Ok(heap_void_ptr(ws))
                })
            }
            _ => fut.cancel_with_err(TYPE_ERR, "not http1".into()),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn http1_h2c_full(fut: *mut FfiFuture, http: *mut DynHttpRequest){
    unsafe{
        let http = *Box::from_raw(http);
        let fut = &*fut;
        
        match http {
            DynHttpRequest::Http1(one) => {
                spawn_task_with(fut, async move {
                    let h2 = one.h2c_full(None).await?;
                    Ok(heap_void_ptr(h2))
                })
            }
            _ => fut.cancel_with_err(TYPE_ERR, "not http1".into()),
        }
    }
}
