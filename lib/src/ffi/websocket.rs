use std::ptr;

use httprs_core::ffi::{futures::FfiFuture, slice::FfiSlice};
use tokio::io::{BufReader, ReadHalf, WriteHalf};

use http::{shared::Stream, websocket::{core::WebSocketFrame, socket::WebSocket}};

use crate::{ffi::utils::heap_void_ptr, spawn_task_with};


pub type DynWebSocket = WebSocket<BufReader<ReadHalf<Box<dyn Stream>>>, WriteHalf<Box<dyn Stream>>>;


#[repr(C)]
pub struct FfiWsFrame{
    pub fin: bool,
    pub rsv: u8,
    pub opcode: u8,
    pub masked: bool,
    pub payload: FfiSlice,
}
impl FfiWsFrame{
    pub fn from_owned(mut frame: WebSocketFrame) -> Self{
        frame.unmask_in_place();
        Self { 
            fin: frame.fin, 
            rsv: frame.rsv, 
            opcode: frame.opcode.into(), 
            masked: frame.masked,
            payload: frame.source[frame.payload].to_vec().into(),
        }
    }
}


#[unsafe(no_mangle)]
pub extern "C" fn websocket_read_frame(fut: *mut FfiFuture, ws: *mut DynWebSocket){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            Ok(heap_void_ptr(FfiWsFrame::from_owned(ws.read_frame().await?)))
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_free_frame(frame: *mut FfiWsFrame){
    unsafe{
        drop(Box::from_raw(frame))
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_flush(fut: *mut FfiFuture, ws: *mut DynWebSocket){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.flush().await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_free(ws: *mut DynWebSocket){
    unsafe{
        drop(Box::from_raw(ws))
    }
}



#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_continuation(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.send_continuation(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_continuation_masked(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;
        let mut mask = [0u8; 4];
        rand::fill(&mut mask);

        spawn_task_with(fut, async move{
            ws.send_continuation_masked(&mask, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_continuation_frag(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.send_continuation_frag(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_continuation_masked_frag(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;
        let mut mask = [0u8; 4];
        rand::fill(&mut mask);

        spawn_task_with(fut, async move{
            ws.send_continuation_masked_frag(&mask, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_text(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.send_text(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_text_masked(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;
        let mut mask = [0u8; 4];
        rand::fill(&mut mask);

        spawn_task_with(fut, async move{
            ws.send_text_masked(&mask, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_text_frag(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.send_text_frag(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_text_masked_frag(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;
        let mut mask = [0u8; 4];
        rand::fill(&mut mask);

        spawn_task_with(fut, async move{
            ws.send_text_masked_frag(&mask, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_binary(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.send_binary(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_binary_masked(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;
        let mut mask = [0u8; 4];
        rand::fill(&mut mask);

        spawn_task_with(fut, async move{
            ws.send_binary_masked(&mask, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_binary_frag(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.send_binary_frag(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_binary_masked_frag(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;
        let mut mask = [0u8; 4];
        rand::fill(&mut mask);

        spawn_task_with(fut, async move{
            ws.send_binary_masked_frag(&mask, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_close(fut: *mut FfiFuture, ws: *mut DynWebSocket, code: u16, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.send_close(code, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_close_masked(fut: *mut FfiFuture, ws: *mut DynWebSocket, code: u16, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;
        let mut mask = [0u8; 4];
        rand::fill(&mut mask);

        spawn_task_with(fut, async move{
            ws.send_close_masked(&mask, code, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_ping(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.send_ping(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_ping_masked(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;
        let mut mask = [0u8; 4];
        rand::fill(&mut mask);

        spawn_task_with(fut, async move{
            ws.send_ping_masked(&mask, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_pong(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;

        spawn_task_with(fut, async move{
            ws.send_pong(buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn websocket_send_pong_masked(fut: *mut FfiFuture, ws: *mut DynWebSocket, buf: FfiSlice){
    unsafe{
        let ws = &mut *ws;
        let fut = &*fut;
        let mut mask = [0u8; 4];
        rand::fill(&mut mask);

        spawn_task_with(fut, async move{
            ws.send_pong_masked(&mask, buf.as_bytes()).await?;
            Ok(ptr::null_mut())
        });
    }
}
