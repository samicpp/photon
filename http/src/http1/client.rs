use std::collections::HashMap;

use rand::Rng;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};

use crate::{http1::get_chunk, http2::{PREFACE, core::Http2Settings, session::Http2Session}, shared::{HttpMethod, HttpRequest, HttpResponse, HttpType, HttpVersion, LibError, LibResult, ReadStream, Stream, WriteStream}, websocket::socket::{MAGIC, WebSocket}};

use base64::{Engine, engine::general_purpose::STANDARD as b64std};

use sha1::{Sha1, Digest};


#[derive(Debug)]
pub struct Http1Request<R: ReadStream, W: WriteStream>{
    pub netr: BufReader<R>,
    pub netw: W,

    pub response: HttpResponse,
    pub line_buf: Vec<u8>,

    pub sent_head: bool,
    pub sent: bool,
    
    pub path: String,
    pub method: HttpMethod,
    pub version: HttpVersion,
    pub headers: HashMap<String, Vec<String>>,
}


impl<S:Stream> Http1Request<ReadHalf<S>, WriteHalf<S>>{
    pub fn new(net: S, bufsize: usize) -> Self{
        let (netr, netw) = tokio::io::split(net);
        let bufr = BufReader::with_capacity(bufsize, netr);
        Self::with_split(bufr, netw)
    }
}
impl<R: ReadStream, W: WriteStream> Http1Request<R, W>{
    pub fn with_split(netr: BufReader<R>, netw: W) -> Self {
        Self {
            netr, netw,

            response: Default::default(),
            line_buf: Vec::new(),
            
            sent_head: false,
            sent: false,

            path: "/".to_owned(),
            method: HttpMethod::Get,
            version: HttpVersion::Http11,
            headers: HashMap::new(),
        }
    }

    pub fn add_header(&mut self, header: &str, value: &str) {
        if let Some(hs) = self.headers.get_mut(header) { hs.push(value.to_owned()); }
        else { self.headers.insert(header.to_owned(), vec![ value.to_owned() ]); }
    }
    pub fn set_header(&mut self, header: &str, value: &str){
        self.headers.insert(header.to_owned(), vec![ value.to_owned() ]);
    }
    pub fn del_header(&mut self, header: &str) -> Option<Vec<String>>{
        self.headers.remove(header)
    }

    pub async fn send_head(&mut self) -> LibResult<()> {
        if !self.sent_head && self.version == HttpVersion::Http09 {
            let head = format!("GET {}\r\n", &self.path);
            self.netw.write_all(head.as_bytes()).await?;
            self.sent_head = true;
            self.response.vcs_complete = true;
            self.response.head_complete = true;
            Ok(())
        }
        else if !self.sent_head{
            let headers = self.headers.iter().map(|(h,vs)|vs.iter().map(|v| format!("{}: {}\r\n", h, v)).collect::<String>()).collect::<String>();
            let head = format!(
                "{} {} {}\r\n{}\r\n", 
                match &self.method { HttpMethod::Unknown(Some(s)) => s.to_owned(), v => format!("{}", v)},
                &self.path,
                match &self.version { HttpVersion::Unknown(Some(s)) => s.to_owned(), v => format!("{}", v)},
                headers,
            );
            
            self.netw.write_all(head.as_bytes()).await?;
            self.sent_head = true;

            Ok(())
        }
        else{
            Err(LibError::ConnectionClosed)
        }
    }

    pub async fn write(&mut self, body: &[u8]) -> LibResult<()> {
        if !self.sent && self.version == HttpVersion::Http09 {
            if !self.sent_head { self.send_head().await? }
            Ok(())
        }
        else if !self.sent{
            if !self.sent_head{
                self.headers.insert("Transfer-Encoding".to_owned(), vec!["chunked".to_owned()]);
                self.send_head().await?;
            }
            self.netw.write_all(&get_chunk(body)).await?;
            Ok(())
        }
        else{
            Err(LibError::ConnectionClosed)
        }
    }
    pub async fn send(&mut self, body: &[u8]) -> LibResult<()> {
        if !self.sent && self.version == HttpVersion::Http09 {
            if !self.sent_head { self.send_head().await? }
            self.sent = true;
            Ok(())
        }
        else if !self.sent_head{
            self.headers.insert("Content-Length".to_owned(), vec![body.len().to_string()]);
            self.send_head().await?;
            self.netw.write_all(body).await?;
            self.sent = true;
            Ok(())
        }
        else if !self.sent{
            self.netw.write_all(&get_chunk(body)).await?;
            self.netw.write_all(b"0\r\n\r\n").await?;
            Ok(())
        }
        else{
            Err(LibError::ConnectionClosed)
        }
    }
    pub async fn flush(&mut self) -> LibResult<()> {
        self.netw.flush().await.map_err(|e| e.into())
    }

    pub async fn read_response(&mut self) -> LibResult<&HttpResponse> {
        self.line_buf.clear();

        if !self.response.valid {

        }
        else if !self.response.vcs_complete && self.version != HttpVersion::Http09 {
            let _ = self.netr.read_until(b'\n', &mut self.line_buf).await?;

            let fullstr = String::from_utf8_lossy(&self.line_buf);
            let fullstr = fullstr.trim_end_matches(['\r', '\n']);
            let vcs: Vec<&str> = fullstr.splitn(3, ' ').collect();
            
            if vcs.len() != 3{
                self.response.valid = false;
            }
            else{
                self.response.version = // should always be same version as client
                if vcs[0].eq_ignore_ascii_case("http/1.0") { HttpVersion::Http10 }
                else if vcs[0].eq_ignore_ascii_case("http/1.1") { HttpVersion::Http11 }
                else { HttpVersion::Unknown(Some(vcs[0].to_owned())) };
                self.response.code = vcs[1].parse().unwrap_or(0);
                self.response.status = vcs[2].to_owned();
            }

            self.response.vcs_complete = true;
        }
        else if !self.response.head_complete && self.version != HttpVersion::Http09 {
            let _ = self.netr.read_until(b'\n', &mut self.line_buf).await?;
            let fullstr = String::from_utf8_lossy(&self.line_buf);
            let hv: Vec<&str> = fullstr.splitn(2, ':').map(|e|e.trim()).collect();

            if fullstr.trim().is_empty(){
                self.response.head_complete = true;
            }
            else if hv.len() == 1 {
                self.response.valid = false;
            }
            else{
                if let Some(hs) = self.response.headers.get_mut(&hv[0].to_ascii_lowercase()) { hs.push(hv[1].to_owned()); }
                else { self.response.headers.insert(hv[0].to_ascii_lowercase(), vec![ hv[1].to_owned() ]); }
            }
        }
        else if !self.response.body_complete {
            if let Some(te) = self.response.headers.get("transfer-encoding") && te[0].contains("chunked") {
                let _ = self.netr.read_until(b'\n', &mut self.line_buf).await?;
                let string = String::from_utf8_lossy(&self.line_buf);
                let len = match usize::from_str_radix(string.trim(), 16) { Ok(s) => s, Err(_) => 0, };

                if len == 0{
                    self.response.body_complete = true;
                }
                else{
                    let ol = self.response.body.len();
                    self.response.body.resize(self.response.body.len() + len, 0);
                    self.netr.read_exact(&mut self.response.body[ol..]).await?;
                    self.netr.read_until(b'\n', &mut self.line_buf).await?;
                }
            }
            else if let Some(cl) = self.response.headers.get("content-length") && let Ok(len) = cl[0].parse::<usize>(){
                self.response.body.resize(len, 0);
                self.netr.read_exact(&mut self.response.body).await?;
                self.response.body_complete = true;
            }
            else if self.response.version == HttpVersion::Http10 || self.response.version == HttpVersion::Http09 {
                self.netr.read_to_end(&mut self.response.body).await?;
                self.response.body_complete = true;
            }
            else{
                self.response.body_complete = true;
            }
        }
        Ok(&self.response)
    }
    pub async fn read_until_complete(&mut self) -> LibResult<&HttpResponse>{
        while self.response.valid && !self.response.body_complete { let _ = self.read_response().await?; }
        Ok(&self.response)
    }
    pub async fn read_until_head_complete(&mut self) -> LibResult<&HttpResponse>{
        while self.response.valid && !self.response.head_complete { let _ = self.read_response().await?; }
        Ok(&self.response)
    }

    pub fn reset(&mut self){
        self.response.reset();
        self.method = HttpMethod::Get;
        self.path = String::new();
        self.version = HttpVersion::Http11;
        self.headers.clear();
        self.sent_head = false;
        self.sent = false;
    }

    pub async fn websocket_upgrade(&mut self, key: &[u8]) -> LibResult<String> {
        if key.len() != 16 { return Err(LibError::Invalid); }

        let wskey = b64std.encode(key);
        
        self.set_header("Connection", "upgrade");
        self.set_header("Upgrade", "websocket");
        self.set_header("Sec-WebSocket-Version", "13");
        self.set_header("Sec-WebSocket-Key", &wskey);

        self.send(b"").await?;
        Ok(wskey)
    }
    pub fn websocket_direct(self) -> WebSocket<BufReader<R>, W> {
        WebSocket::with_split(self.netr, self.netw)
    }
    pub async fn websocket_unchecked(mut self) -> LibResult<WebSocket<BufReader<R>, W>> {
        let mut key = [0; 16];
        rand::rng().fill(&mut key);
        let _ = self.websocket_upgrade(&key).await?;
        
        Ok(self.websocket_direct())
    }
    pub async fn websocket_lazy(mut self) -> LibResult<WebSocket<BufReader<R>, W>>{
        let mut key = [0; 16];
        rand::rng().fill(&mut key);
        let _ = self.websocket_upgrade(&key).await?;
        
        let res = self.read_until_head_complete().await?;
        if res.code != 101 {
            Err(LibError::NotAccepted)
        }
        else {
            Ok(self.websocket_direct())
        }
    }
    pub async fn websocket_strict(mut self) -> LibResult<WebSocket<BufReader<R>, W>>{
        let mut key = [0; 16];
        rand::rng().fill(&mut key);
        let bkey = self.websocket_upgrade(&key).await?;

        let mut sha = Sha1::new();
        sha.update(bkey.as_bytes());
        sha.update(MAGIC);
        let acckey = b64std.encode(sha.finalize());
        
        let res = self.read_until_head_complete().await?;
        if res.code != 101 {
            Err(LibError::NotAccepted)
        }
        else if let Some(reskey) = res.headers.get("sec-websocket-accept") && reskey[0] == acckey {
            Ok(self.websocket_direct())
        }
        else {
            Err(LibError::InvalidUpgrade)
        }
    }

    pub fn http2_direct(self, settings: Http2Settings) -> Http2Session<BufReader<R>, W> {
        Http2Session::with(self.netr, self.netw, crate::http2::session::Mode::Client, true, settings)
    }
    pub async fn http2_prior_knowledge(mut self) -> LibResult<Http2Session<BufReader<R>, W>> {
        if self.sent { return Err(LibError::ConnectionClosed); }
        if self.sent_head { return Err(LibError::HeadersSent); }
        
        self.netw.write_all(PREFACE).await?;
        Ok(self.http2_direct(Http2Settings::default_no_push()))
    }
    pub async fn h2c_upgrade(&mut self, settings: Option<Http2Settings>, body: &[u8]) -> LibResult<()> {
        // connection can still be answered normally

        if self.sent { return Err(LibError::ConnectionClosed); }
        if self.sent_head { return Err(LibError::HeadersSent); }

        if let Some(settings) = settings {
            self.set_header("Connection", "Upgrade, HTTP2-Settings");
            self.set_header("Upgrade", "h2c");
            self.set_header("HTTP2-Settings", &b64std.encode(settings.to_vec()));
        }
        else {
            self.set_header("Upgrade", "h2c");
            self.set_header("Connection", "Upgrade");
        }
        
        self.send(body).await
    }
    pub async fn h2c_full(mut self, settings: Option<Http2Settings>) -> LibResult<Http2Session<BufReader<R>, W>> {
        self.h2c_upgrade(settings, b"").await?;

        let res = self.read_until_head_complete().await?;
        
        if res.code == 101 {
            let settings = 
            if let Some(settings) = res.headers.get("http2-settings"){
                let buff = b64std.decode(&settings[0]).unwrap_or(vec![]);
                Http2Settings::from(&buff)
            }
            else {
                Http2Settings::default_no_push()
            };

            Ok(self.http2_direct(settings))
        }
        else {
            Err(LibError::NotAccepted)
        }
    }
}
impl<R: ReadStream, W: WriteStream> HttpRequest for Http1Request<R, W>{
    #[inline]
    fn get_type(&self) -> HttpType {
        HttpType::Http1
    }

    #[inline]
    fn get_response(&self) -> &HttpResponse {
        &self.response
    }
    #[inline]
    fn read_response(&'_ mut self) -> impl Future<Output = Result<&'_ HttpResponse, LibError>> + Send + '_ {
        self.read_response()
    }
    #[inline]
    fn read_until_complete(&'_ mut self) -> impl Future<Output = Result<&'_ HttpResponse, LibError>> + Send + '_ {
        self.read_until_complete()
    }
    #[inline]
    fn read_until_head_complete(&'_ mut self) -> impl Future<Output = Result<&'_ HttpResponse, LibError>> + Send + '_ {
        self.read_until_head_complete()
    }


    #[inline] fn add_header(&mut self, header: &str, value: &str) { self.add_header(header, value) }
    #[inline] fn set_header(&mut self, header: &str, value: &str){ self.set_header(header, value) }
    #[inline] fn del_header(&mut self, header: &str) -> Option<Vec<String>>{ self.del_header(header) }

    #[inline]
    fn set_method(&mut self, method: HttpMethod) {
        self.method = method;
    }
    #[inline]
    fn set_scheme(&mut self, _: String) {
        ()
    }
    #[inline]
    fn set_path(&mut self, path: String) {
        self.path = path;
    }
    #[inline]
    fn set_host(&mut self, host: String) {
        self.set_header(&"Host", &host);
    }

    #[inline]
    fn write<'a>(&'a mut self, body: &'a [u8] ) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.write(body)
    }
    #[inline]
    fn send<'a>(&'a mut self, body: &'a [u8] ) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.send(body)
    }
    #[inline]
    fn flush<'a>(&'a mut self) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.flush()
    }
}

