use std::{cmp::min, io, sync::{Arc, Mutex as SyncMutex, atomic::{AtomicBool, Ordering}}};

use dashmap::DashMap;
use tokio::{io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf}, sync::{Mutex as AsyncMutex, Notify}};

use crate::{http2::{core::{Http2Frame, Http2FrameType, Http2Settings}, hpack::{HeaderType, HpackError, decoder::Decoder, encoder::Encoder}}, shared::{LibError, LibResult, ReadStream, Stream, WriteStream}};

pub const PREFACE: &'static [u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ambiguous,
    Client,
    Server,
}
impl Mode {
    pub fn is_ambiguous(&self) -> bool { if let Self::Ambiguous = self { true } else { false } }
    pub fn is_client(&self) -> bool { if let Self::Client = self { true } else { false } }
    pub fn is_server(&self) -> bool { if let Self::Server = self { true } else { false } }
}

#[derive(Debug)]
pub struct Http2Data {
    pub new: bool,

    pub window: usize,
    pub notify: Arc<Notify>,

    pub stream_id: u32,
    pub reset: bool,

    pub end_head: bool,
    pub end_body: bool,
    pub self_end_head: bool,
    pub self_end_body: bool,
    
    pub body: Vec<u8>,
    pub head: Vec<u8>,
    pub headers: Vec<(Vec<u8>, Vec<u8>)>,

    pub head_complete: Arc<Notify>,
    pub body_received: Arc<Notify>,
    // pub body_complete: Arc<Notify>,

    pub ascociated: Option<u32>,
    pub promising: Option<u32>, 
    pub promise: Vec<u8>,
    pub push_headers: Vec<(Vec<u8>, Vec<u8>)>,

    pub own_window: Option<SyncMutex<usize>>,
}
impl Http2Data {
    pub fn empty(stream_id: u32, sett: Http2Settings) -> Self {
        Self {
            new: true,
            window: sett.initial_window_size.unwrap_or(65535) as usize,
            notify: Arc::new(Notify::new()),
            stream_id,
            reset: false,
            end_head: false,
            end_body: false,
            self_end_head: false,
            self_end_body: false,
            body: Vec::new(),
            head: Vec::new(),
            headers: Vec::new(),
            head_complete: Arc::new(Notify::new()),
            body_received: Arc::new(Notify::new()),
            // body_complete: Arc::new(Notify::new()),
            ascociated: None,
            promising: None,
            promise: Vec::new(),
            push_headers: Vec::new(),
            own_window: None,
        }
    }
}

#[derive(Debug)]
pub struct Http2Session<R: ReadStream, W: WriteStream>{
    pub netr: AsyncMutex<R>,
    pub netw: AsyncMutex<W>,
    pub mode: Mode,
    pub strict: bool,

    pub decoder: AsyncMutex<Decoder<'static>>,
    pub encoder: AsyncMutex<Encoder<'static>>,

    pub max_stream_id: SyncMutex<u32>,
    pub streams: DashMap<u32, Http2Data>,

    pub goaway: AtomicBool,
    pub goaway_frame: SyncMutex<Option<Http2Frame>>,

    pub window: SyncMutex<usize>,
    pub notify: Notify,

    pub settings: SyncMutex<Http2Settings>,

    // TODO: force other side to respect settings & flow control
    pub own_settings: Option<SyncMutex<Http2Settings>>,
    pub own_window: Option<SyncMutex<usize>>,
}
impl<S: Stream> Http2Session<ReadHalf<S>, WriteHalf<S>> {
    pub fn new(net: S) -> Self {
        let (netr, netw) = tokio::io::split(net);
        Self::with(netr, netw, Mode::Ambiguous, true, Http2Settings::default())
    }
    pub fn new_client(net: S) -> Self {
        let (netr, netw) = tokio::io::split(net);
        Self::with(netr, netw, Mode::Client, true, Http2Settings::default())
    }
    pub fn new_server(net: S) -> Self {
        let (netr, netw) = tokio::io::split(net);
        Self::with(netr, netw, Mode::Server, true, Http2Settings::default())
    }
}
impl<R: ReadStream, W: WriteStream> Http2Session<R, W> {
    pub fn with(netr: R, netw: W, mode: Mode, strict: bool, settings: Http2Settings) -> Self {
        let netr = AsyncMutex::new(netr);
        let netw = AsyncMutex::new(netw);

        Self {
            netr, netw, mode, strict,
            decoder: AsyncMutex::new(Decoder::new(settings.header_table_size.unwrap_or(4096) as usize)),
            encoder: AsyncMutex::new(Encoder::new(settings.header_table_size.unwrap_or(4096) as usize)),
            max_stream_id: SyncMutex::new(0),
            streams: DashMap::new(),
            goaway: AtomicBool::new(false),
            goaway_frame: SyncMutex::new(None),
            window: SyncMutex::new(settings.initial_window_size.unwrap_or(65535) as usize),
            notify: Notify::new(),
            settings: SyncMutex::new(settings),
            own_settings: None,
            own_window: None,
        }
    }

    

    pub async fn send_preface(&self) -> io::Result<()> {
        self.netw.lock().await.write_all(PREFACE).await
    }
    pub async fn read_preface(&self) -> io::Result<bool> {
        let mut pre = [0; PREFACE.len()];
        self.netr.lock().await.read_exact(&mut pre).await?;
        Ok(pre == PREFACE)
    }


    pub async fn read_frame(&self) -> io::Result<Http2Frame> {
        let mut reader = self.netr.lock().await;
        Http2Frame::from_reader(&mut *reader).await
    }
    pub async fn read_until(&self, frame_type: Http2FrameType) -> io::Result<Vec<Http2Frame>> {
        let mut frames = vec![];
        let mut done = false;

        while !done {
            let frame = self.read_frame().await?;
            
            if frame.ftype == frame_type { done = true }

            frames.push(frame);
        }

        Ok(frames)
    }
    pub async fn read_until_not(&self, frame_type: Http2FrameType) -> io::Result<Vec<Http2Frame>> {
        let mut frames = vec![];
        let mut done = false;

        while !done {
            let frame = self.read_frame().await?;
            
            if frame.ftype != frame_type { done = true }

            frames.push(frame);
        }
        
        Ok(frames)
    }

    pub async fn next(&self) -> LibResult<Option<u32>> {
        let frame = self.read_frame().await?;

        // println!("\x1b[36m{:?}\x1b[0m {:?}", frame.ftype, frame.source);
        
        self.handle(frame).await
    }
    pub async fn next_until(&self, frame_type: Http2FrameType) -> LibResult<Vec<u32>> {
        let mut opened = vec![];
        let mut done = false;

        while !done {
            let frame = self.read_frame().await?;
            
            if frame.ftype == frame_type { done = true }

            if let Some(open) = self.handle(frame).await? {
                opened.push(open);
            }
        }
        
        Ok(opened)
    }
    pub async fn next_until_not(&self, frame_type: Http2FrameType) -> LibResult<Vec<u32>> {
        let mut opened = vec![];
        let mut done = false;

        while !done {
            let frame = self.read_frame().await?;
            
            if frame.ftype != frame_type { done = true }

            if let Some(open) = self.handle(frame).await? {
                opened.push(open);
            }
        }
        
        Ok(opened)
    }


    pub async fn handle(&self, frame: Http2Frame) -> LibResult<Option<u32>> {
        // TODO: strict check wether frame fields are valid, e.g. allowed flags

        {
            let mut msid = self.max_stream_id.lock().unwrap();
            if frame.stream_id > *msid {
                *msid = frame.stream_id
            }
        }

        match frame.ftype {
            Http2FrameType::Data => {                
                if let Some(mut shard) = self.streams.get_mut(&frame.stream_id) {
                    // TODO: strict check wether closed

                    shard.body.extend_from_slice(frame.get_payload());
                    
                    if frame.is_end_stream() { shard.end_body = true }

                    shard.body_received.notify_waiters();

                    drop(shard);
                    self.send_window_update(0, frame.length).await?;
                    self.send_window_update(frame.stream_id, frame.length).await?;

                    Ok(None)
                }
                else {
                    Err(LibError::InvalidStream)
                }
            },
            Http2FrameType::Headers => {
                let mut decoder = self.decoder.lock().await;
                match self.streams.get_mut(&frame.stream_id) {
                    // TODO: allow leading or trailing non opening headers
                    // TODO: strict verify that this is the case
                    Some(mut shard) if self.mode.is_client() || self.mode.is_ambiguous() => {

                        shard.head.extend_from_slice(frame.get_payload());

                        if frame.is_end_headers() {
                            let mut dec = decoder.decode_all(&shard.head).ok_or(HpackError::InvalidHeaderField)?;
                            shard.headers.append(&mut dec);
                            shard.end_head = true;
                            shard.head.clear();
                            shard.head_complete.notify_waiters();
                        }
                        if frame.is_end_stream() { shard.end_body = true }
                        Ok(None)
                    },
                    None if self.mode.is_server() || self.mode.is_ambiguous() => {
                        let mut stream = Http2Data::empty(frame.stream_id, *self.settings.lock().unwrap());

                        stream.head.extend_from_slice(frame.get_payload());

                        if frame.is_end_stream() { stream.end_body = true }
                        if frame.is_end_headers() {
                            stream.end_head = true;
                            stream.headers.append(&mut decoder.decode_all(&stream.head).ok_or(HpackError::InvalidHeaderField)?);
                            stream.head.clear();
                        }
                        
                        self.streams.insert(frame.stream_id, stream);
                        
                        if frame.is_end_headers() {
                            Ok(Some(frame.stream_id))
                        }
                        else {
                            Ok(None)
                        }
                    }
                    _ => Err(LibError::ProtocolError),
                }
            },
            Http2FrameType::Priority => {
                Ok(None)
            },
            Http2FrameType::RstStream => {
                if let Some(mut shard) = self.streams.get_mut(&frame.stream_id) {
                    shard.reset = true;
                    shard.notify.notify_waiters();

                    Ok(None)
                }
                else {
                    Err(LibError::InvalidStream)
                }
            },
            Http2FrameType::Settings => {
                // TODO: strict only allow known settings (1 - 6) and stream_id == 0
                if !frame.is_ack() {
                    {
                        let sett = Http2Settings::from(frame.get_payload());
                        let mut settings = self.settings.lock().unwrap();
                        
                        if let Some(val) = sett.header_table_size { settings.header_table_size = Some(val) }
                        if let Some(val) = sett.enable_push { settings.enable_push = Some(val) }
                        if let Some(val) = sett.max_concurrent_streams { settings.max_concurrent_streams = Some(val) }
                        if let Some(val) = sett.initial_window_size { settings.initial_window_size = Some(val) }
                        if let Some(val) = sett.max_frame_size { settings.max_frame_size = Some(val) }
                        if let Some(val) = sett.max_header_list_size { settings.max_header_list_size = Some(val) }
                    }

                    self.write_frame(Http2FrameType::Settings, 1, 0, None, None, None).await?;
                }

                Ok(None)
            },
            Http2FrameType::PushPromise => {
                let pay = frame.get_payload();

                // TODO: strict verify associated stream exists
                if (self.mode.is_client() || self.mode.is_ambiguous()) && pay.len() >= 4 {
                    let mut decoder = self.decoder.lock().await;

                    let promised = u32::from_be_bytes([pay[0], pay[1], pay[2], pay[3]]);
                    if self.streams.contains_key(&promised) {
                        Err(LibError::ProtocolError)
                    }
                    else if let Some(mut shard) = self.streams.get_mut(&frame.stream_id) {
                        let mut stream = Http2Data::empty(promised, *self.settings.lock().unwrap());

                        stream.promise.extend_from_slice(&pay[4..]);

                        if frame.is_end_headers() {
                            stream.push_headers.append(&mut decoder.decode_all(&stream.promise).ok_or(HpackError::InvalidHeaderField)?);
                            stream.promise.clear();
                        }
                        if frame.is_end_stream() { stream.end_body = true }

                        self.streams.insert(promised, stream);
                        shard.promising = Some(promised);

                        if frame.is_end_headers() {
                            Ok(Some(promised))
                        }
                        else {
                            Ok(None)
                        }
                    }
                    else {
                        Err(LibError::ProtocolError)
                    }
                }
                else {
                    Err(LibError::ProtocolError)
                }
            },
            Http2FrameType::Ping => {
                if !frame.is_ack() { self.send_ping(true, frame.get_payload()).await?; }
                Ok(None)
            },
            Http2FrameType::Goaway => {
                self.goaway.store(true, Ordering::SeqCst);
                let mut goaway = self.goaway_frame.lock().unwrap();
                *goaway = Some(frame);
                self.notify.notify_waiters();
                Ok(None)
            },
            Http2FrameType::WindowUpdate => {
                let pay = frame.get_payload();
                
                if pay.len() == 4 {
                    let size = u32::from_be_bytes([pay[0], pay[1], pay[2], pay[3]]);
                    if frame.stream_id == 0 {
                        let mut window = self.window.lock().unwrap();
                        *window += size as usize;
                        self.notify.notify_waiters();
                        Ok(None)
                    }
                    else if let Some(mut shard) = self.streams.get_mut(&frame.stream_id) {
                        shard.window += size as usize;
                        shard.notify.notify_waiters();
                        Ok(None)
                    }
                    else {
                        Err(LibError::InvalidStream)
                    }
                }
                else {
                    Err(LibError::ProtocolError)
                }
            },
            Http2FrameType::Continuation => {
                let mut decoder = self.decoder.lock().await;
                // TODO: strict verify wether headers has opened
                if let Some(mut shard) = self.streams.get_mut(&frame.stream_id) {
                    if let Some(promising) = shard.promising && let Some(mut promised) = self.streams.get_mut(&promising) {
                        promised.promise.extend_from_slice(frame.get_payload());

                        if frame.is_end_headers() {
                            let mut dec = decoder.decode_all(&promised.promise).ok_or(HpackError::InvalidHeaderField)?;
                            promised.push_headers.append(&mut dec);
                            promised.promise.clear();
                            shard.promising = None;
                            Ok(Some(promising))
                        }
                        else {
                            Ok(None)
                        }
                    }
                    else {
                        shard.head.extend_from_slice(frame.get_payload());

                        if frame.is_end_stream() { shard.end_body = true }
                        if frame.is_end_headers() {
                            let mut dec = decoder.decode_all(&shard.head).ok_or(HpackError::InvalidHeaderField)?;
                            shard.headers.append(&mut dec);
                            shard.end_head = true;
                            shard.head.clear();
                            shard.head_complete.notify_waiters();

                            if shard.new {
                                Ok(Some(frame.stream_id))
                            }
                            else {
                                Ok(None)
                            }
                        }
                        else {
                            Ok(None)
                        }
                    }
                }
                else {
                    Err(LibError::InvalidStream)
                }
            },
            Http2FrameType::Invalid(_) => {
                if self.strict {
                    Err(LibError::ProtocolError)
                }
                else {
                    Ok(None)
                }
            }
        }
    }

    pub fn open_stream(&self) -> Option<u32> {
        let mut max_id = self.max_stream_id.lock().unwrap();
        let stream_id = 
        if self.mode.is_ambiguous() { 
            *max_id + 1
        }
        else if self.mode.is_client() {
            if *max_id % 2 == 1 { *max_id + 2 }
            else { *max_id + 1 }
        }
        else {
            if *max_id % 2 == 0 { *max_id + 2 }
            else { *max_id + 1 }
        }
        ;
        if self.settings.lock().unwrap().max_concurrent_streams.map(|v| stream_id > v).unwrap_or(true) {
            *max_id = stream_id;
            Some(stream_id)
        }
        else {
            None
        }
    }



    pub async fn write_frame(&self, ftype: Http2FrameType, flags: u8, stream_id: u32, priority: Option<&[u8]>, payload: Option<&[u8]>, padding: Option<&[u8]>) -> io::Result<()> {
        self.netw.lock().await.write_all(&Http2Frame::create(ftype, flags, stream_id, priority, payload, padding)).await
    }

    pub async fn send_data(&self, stream_id: u32, end: bool, buf: &[u8]) -> LibResult<()> {
        // let mut stream = 
        let notify =
        if let Some(mut shard) = self.streams.get_mut(&stream_id) {
            if shard.self_end_body || shard.reset {
                return Err(LibError::StreamClosed)
            }

            shard.self_end_body = end;
            shard.notify.clone()
        }
        else {
            return Err(LibError::InvalidStream)
        };

        if buf.len() == 0 {
            if end {
                self.write_frame(Http2FrameType::Data, 1, stream_id, None, None, None).await?;
            }
            return Ok(());
        }

        let mut pos = 0;
        let mut buff = vec![];
        let mfs = self.settings.lock().unwrap().max_frame_size.unwrap_or(16384) as usize;
        // let mut window = self.window.lock().unwrap();
        // let mut minim = min(mfs, min(*window, stream.window));

        // while buf.len() - pos > minim {
        while buf.len() > pos {
            let (max, ncws, nsws) =
            {
                let mut window = self.window.lock().unwrap();
                let mut stream = self.streams.get_mut(&stream_id).unwrap();

                if stream.reset {
                    return Err(LibError::ResetStream)
                }

                let max = min(buf.len() - pos, min(*window, stream.window));
                *window -= max;
                stream.window -= max;
                // drop(window);
                // drop(stream);
                (max, *window, stream.window)
            };


            if max > 0 {
                let chunks = max / mfs;
                let rem = max % mfs;

                for _ in 0..chunks {
                    let end_pos = pos + mfs;
                    
                    if end_pos == buf.len() {
                        buff.append(&mut Http2Frame::create(Http2FrameType::Data, if end { 1 } else { 0 }, stream_id, None, Some(&buf[pos..end_pos]), None));
                    }
                    else {
                        buff.append(&mut Http2Frame::create(Http2FrameType::Data, 0, stream_id, None, Some(&buf[pos..end_pos]), None));
                    }

                    pos += mfs;
                }


                let end_pos = pos + rem;

                if end_pos == buf.len() {
                    buff.append(&mut Http2Frame::create(Http2FrameType::Data, if end { 1 } else { 0 }, stream_id, None, Some(&buf[pos..end_pos]), None));
                }
                else {
                    buff.append(&mut Http2Frame::create(Http2FrameType::Data, 0, stream_id, None, Some(&buf[pos..end_pos]), None));
                }

                pos += rem;
                
                
                self.netw.lock().await.write_all(&buff).await?;
                buff.clear();
            }

            if nsws == 0 {
                notify.notified().await;
            }
            else if ncws == 0 {
                self.notify.notified().await;
            }
            

            // window = self.window.lock().unwrap();
            // stream = self.streams.get_mut(&stream_id).unwrap();
        }

        Ok(())
    }

    pub async fn send_headers(&self, stream_id: u32, end: bool, headers: &[(&[u8], &[u8])]) -> LibResult<()> {
        {
            let mut shard = 
            match self.streams.get_mut(&stream_id) {
                // doing !self.mode.is_(oposite)() would be more optimized maybe
                Some(s) if self.mode.is_server() || self.mode.is_ambiguous() => s,
                None if self.mode.is_client() || self.mode.is_ambiguous() => {
                    let mut stream = Http2Data::empty(stream_id, *self.settings.lock().unwrap());
                    stream.new = false;

                    self.streams.insert(stream_id, stream);
                    self.streams.get_mut(&stream_id).unwrap()
                }
                _ => return Err(LibError::InvalidStream),
            };

            if shard.self_end_head || shard.self_end_body {
                return Err(LibError::StreamClosed)
            }

            shard.self_end_head = true;
            shard.self_end_body = end;
        }

        let mut hpacke = self.encoder.lock().await;
        let enc = {
            let mut buff = Vec::new();

            for &(nam, val) in headers {
                hpacke.encode(&mut buff, HeaderType::NotIndexed, nam, val, None)?;
            }

            buff
        };
        let mfs = self.settings.lock().unwrap().max_frame_size.unwrap_or(16384) as usize;
        
        let mut pos = 0;
        let mut buff = Vec::with_capacity(9 + enc.len() / mfs * 9 + enc.len());

        
        if enc.len() < mfs {
            buff.append(&mut Http2Frame::create(Http2FrameType::Headers, if end { 5 } else { 4 }, stream_id, None, Some(&enc), None));
        }
        else {
            buff.append(&mut Http2Frame::create(Http2FrameType::Headers, 0, stream_id, None, Some(&enc[pos..pos + mfs]), None));
            pos += mfs;

            let mut chunks = enc.len() / mfs;

            if enc.len() % mfs == 0 {
                chunks -= 1;
                // rem += mfs;
            }

            for _ in 0..chunks {
                buff.append(&mut Http2Frame::create(Http2FrameType::Continuation, 0, stream_id, None, Some(&enc[pos..pos + mfs]), None));
                pos += mfs;
            }

            buff.append(&mut Http2Frame::create(Http2FrameType::Continuation, if end { 5 } else { 4 }, stream_id, None, Some(&enc[pos..]), None));
        }

        self.netw.lock().await.write_all(&buff).await?;
        drop(hpacke);

        Ok(())
    }

    pub async fn send_priority(&self, stream_id: u32, dependency: u32, weight: u8) -> io::Result<()> {
        self.write_frame(Http2FrameType::Priority, 0, stream_id, None, Some(&[(dependency >> 24) as u8, (dependency >> 16) as u8, (dependency >> 8) as u8, dependency as u8, weight]), None).await
    }
    
    pub async fn send_rst_stream(&self, stream_id: u32, code: u32) -> io::Result<()> { 
        self.write_frame(Http2FrameType::RstStream, 0, stream_id, None, Some(&u32::to_be_bytes(code)), None).await
    }

    pub async fn send_settings(&self, settings: Http2Settings) -> io::Result<()> { 
        self.write_frame(Http2FrameType::Settings, 0, 0, None, Some(&settings.to_vec()), None).await
    }
    
    pub async fn send_push_promise(&self, associate_id: u32, promise_id: u32, headers: &[(&[u8], &[u8])]) -> LibResult<()> {
        {
            let mut stream =
            if self.mode.is_client() {
                return Err(LibError::ProtocolError)
            }
            else if self.streams.contains_key(&promise_id) || !self.streams.contains_key(&associate_id) {
                return Err(LibError::InvalidStream)
            }
            else {
                Http2Data::empty(promise_id, *self.settings.lock().unwrap())
            };

            stream.ascociated = Some(associate_id);

            self.streams.insert(promise_id, stream);
        }

        let mut hpacke = self.encoder.lock().await;
        let enc = {
            let mut buff = Vec::new();

            for &(nam, val) in headers {
                hpacke.encode(&mut buff, HeaderType::NotIndexed, nam, val, None)?;
            }

            buff
        };
        let mfs = self.settings.lock().unwrap().max_frame_size.unwrap_or(16384) as usize;
        
        let mut pos = 0;
        let mut buff = Vec::with_capacity(9 + enc.len() / mfs * 9 + enc.len());

        
        if enc.len() + 4 < mfs {
            let mut pay = Vec::with_capacity(enc.len() + 4);
            pay.extend_from_slice(&u32::to_be_bytes(promise_id));
            pay.extend_from_slice(&enc);

            buff.append(&mut Http2Frame::create(Http2FrameType::PushPromise, 4, associate_id, None, Some(&pay), None));
        }
        else {
            let mut pay = Vec::with_capacity(mfs);
            pay.extend_from_slice(&u32::to_be_bytes(promise_id));
            pay.extend_from_slice(&enc[pos..pos + mfs - 4]);

            buff.append(&mut Http2Frame::create(Http2FrameType::PushPromise, 0, associate_id, None, Some(&pay), None));
            pos += mfs - 4;

            let mut chunks = enc.len() / mfs;

            if enc.len() % mfs == 0 {
                chunks -= 1;
                // rem += mfs;
            }

            for _ in 0..chunks {
                buff.append(&mut Http2Frame::create(Http2FrameType::Continuation, 0, associate_id, None, Some(&enc[pos..pos + mfs]), None));
                pos += mfs;
            }

            buff.append(&mut Http2Frame::create(Http2FrameType::Continuation, 4, associate_id, None, Some(&enc[pos..]), None));
        }

        self.netw.lock().await.write_all(&buff).await?;
        drop(hpacke);

        Ok(())
    }
    
    pub async fn send_ping(&self, ack: bool, buf: &[u8]) -> io::Result<()> { 
        self.write_frame(Http2FrameType::Ping, if ack { 1 } else { 0 }, 0, None, Some(buf), None).await
    }
    
    pub async fn send_goaway(&self, stream_id: u32, code: u32, buf: &[u8]) -> io::Result<()> {
        let mut pay = vec![];
        
        pay.extend_from_slice(&u32::to_be_bytes(stream_id));
        pay.extend_from_slice(&u32::to_be_bytes(code));
        pay.extend_from_slice(buf);

        self.write_frame(Http2FrameType::Goaway, 0, 0, None, Some(&pay), None).await
    }
    
    pub async fn send_window_update(&self, stream_id: u32, size: u32) -> io::Result<()> {
        self.write_frame(Http2FrameType::WindowUpdate, 0, stream_id, None, Some(&u32::to_be_bytes(size)), None).await
    }
    
    // pub async fn send_continuation(&self, stream_id: u32) -> io::Result<()> { unimplemented!() } // no reason for this

}