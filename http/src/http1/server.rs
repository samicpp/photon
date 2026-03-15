use std::collections::HashMap;

use std::io;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as b64std;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use crate::http1::get_chunk;
use crate::http2::core::Http2Settings;
use crate::http2::session::{Http2Data, Http2Session};
use crate::shared::{HttpMethod, LibError, LibResult};
use crate::shared::{HttpType, HttpVersion, ReadStream, Stream, WriteStream, HttpClient, HttpSocket};
use crate::websocket::socket::{MAGIC, WebSocket};


pub const H2C_UPGRADE: &'static [u8] = b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: h2c\r\n\r\n";
pub const WS_UPGRADE: &'static [u8] = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ";



#[derive(Debug)]
pub struct Http1Socket<R: ReadStream, W: WriteStream>{
    pub netr: BufReader<R>,
    pub netw: W,

    pub client: HttpClient,
    pub line_buf: Vec<u8>,

    pub sent_head: bool,
    pub closed: bool,
    
    pub code: u16,
    pub status: String,
    pub headers: HashMap<String, Vec<String>>,

    pub version_override: Option<HttpVersion>,
}


impl<S:Stream> Http1Socket<ReadHalf<S>, WriteHalf<S>>{
    pub fn new(net: S, bufsize: usize) -> Self{
        let (netr, netw) = tokio::io::split(net);
        let bufr = BufReader::with_capacity(bufsize, netr);
        Self::with_split(bufr, netw)
    }
}

impl<R: ReadStream, W: WriteStream> Http1Socket<R, W>{
    pub fn with_split(netr: BufReader<R>, netw: W) -> Self {
        Self {
            netr, netw,

            client: Default::default(),
            line_buf: Vec::new(),
            
            sent_head: false,
            closed: false,

            code: 200,
            status: "OK".to_string(),
            headers: HashMap::new(),

            version_override: None,
        }
    }


    pub async fn read_client(&mut self) -> LibResult<&HttpClient>{
        self.line_buf.clear();

        if !self.client.valid {

        }
        else if !self.client.mpv_complete{
            let _ = self.netr.read_until(b'\n', &mut self.line_buf).await?;

            let fullstr = String::from_utf8_lossy(&self.line_buf);
            let fullstr = fullstr.trim_end_matches(['\r', '\n']);
            let mpv: Vec<&str> = fullstr.splitn(3, ' ').collect();

            if mpv.len() == 2 && mpv[0].eq_ignore_ascii_case("get"){
                self.client.method = HttpMethod::Get;
                self.client.path = mpv[1].to_owned();
                self.client.version = HttpVersion::Http09;

                self.client.head_complete = true;
                self.client.body_complete = true;
            }
            else if mpv.len() != 3{
                self.client.valid = false;
            }
            else{
                self.client.method = HttpMethod::from(mpv[0]);
                self.client.path = mpv[1].to_owned();
                self.client.version = 
                if mpv[2].eq_ignore_ascii_case("http/1.0") { HttpVersion::Http10 }
                else if mpv[2].eq_ignore_ascii_case("http/1.1") { HttpVersion::Http11 }
                else { HttpVersion::Unknown(Some(mpv[2].to_owned())) };
            }

            self.client.mpv_complete = true;
        }
        else if !self.client.head_complete{
            let _ = self.netr.read_until(b'\n', &mut self.line_buf).await?;
            let fullstr = String::from_utf8_lossy(&self.line_buf);
            let hv: Vec<&str> = fullstr.splitn(2, ':').map(|e|e.trim()).collect();

            if fullstr.trim().is_empty(){
                self.client.head_complete = true;
            }
            else if hv.len() == 1 {
                self.client.valid = false;
            }

            else if hv[0].eq_ignore_ascii_case("host"){
                let _ = self.client.host.get_or_insert(hv[1].to_owned());
            }
            else{
                if let Some(hs) = self.client.headers.get_mut(&hv[0].to_ascii_lowercase()) { hs.push(hv[1].to_owned()); }
                else { self.client.headers.insert(hv[0].to_ascii_lowercase(), vec![ hv[1].to_owned() ]); }
            }
        }
        else if !self.client.body_complete{
            if let Some(te) = self.client.headers.get("transfer-encoding") && te[0].contains("chunked") {
                let _ = self.netr.read_until(b'\n', &mut self.line_buf).await?;
                let string = String::from_utf8_lossy(&self.line_buf);
                let len = match usize::from_str_radix(string.trim(), 16) { Ok(s) => s, Err(_) => 0, };

                if len == 0{
                    self.client.body_complete = true;
                }
                else{
                    let ol = self.client.body.len();
                    self.client.body.resize(self.client.body.len() + len, 0);
                    self.netr.read_exact(&mut self.client.body[ol..]).await?;
                    self.netr.read_until(b'\n', &mut self.line_buf).await?;
                }
            }
            else if let Some(cl) = self.client.headers.get("content-length") && let Ok(len) = cl[0].parse::<usize>(){
                self.client.body.resize(len, 0);
                self.netr.read_exact(&mut self.client.body).await?;
                self.client.body_complete = true;
            }
            else if self.client.version == HttpVersion::Http10 {
                self.netr.read_to_end(&mut self.client.body).await?;
                self.client.body_complete = true;
            }
            else{
                self.client.body_complete = true;
            }
        }

        Ok(&self.client)
    }
    pub async fn read_until_complete(&mut self) -> LibResult<&HttpClient>{
        while self.client.valid && !self.client.body_complete { let _ = self.read_client().await?; }
        Ok(&self.client)
    }
    pub async fn read_until_head_complete(&mut self) -> LibResult<&HttpClient>{
        while self.client.valid && !self.client.head_complete { let _ = self.read_client().await?; }
        Ok(&self.client)
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
        if !self.sent_head && self.client.version == HttpVersion::Http09 {
            self.sent_head = true;
            Ok(())
        }
        else if !self.sent_head{
            let headers = self.headers.iter().map(|(h,vs)|vs.iter().map(|v| format!("{}: {}\r\n", h, v)).collect::<String>()).collect::<String>();
            let head = format!(
                "{} {} {}\r\n{}\r\n", 
                if let Some(ov) = &self.version_override { ov.to_string_unknown() }
                else { self.client.version.to_string_unknown() },
                self.code,
                &self.status,
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

    pub async fn write(&mut self, body: &[u8]) -> LibResult<()>{
        if !self.closed && self.client.version == HttpVersion::Http09 {
            if !self.sent_head { self.send_head().await? }
            self.netw.write_all(body).await?;
            Ok(())
        }
        else if !self.closed{
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
    pub async fn close(&mut self, body: &[u8]) -> LibResult<()>{
        if !self.sent_head{
            self.headers.insert("Content-Length".to_owned(), vec![body.len().to_string()]);
            self.send_head().await?;
            self.netw.write_all(body).await?;
            self.closed = true;
            Ok(())
        }
        else if !self.closed && self.client.version == HttpVersion::Http09 {
            self.netw.write_all(body).await?;
            Ok(())
        }
        else if !self.closed{
            self.netw.write_all(&get_chunk(body)).await?;
            self.netw.write_all(b"0\r\n\r\n").await?;
            self.closed = true;
            Ok(())
        }
        else{
            Err(LibError::ConnectionClosed)
        }
    }
    pub async fn flush(&mut self) -> LibResult<()> {
        self.netw.flush().await.map_err(|e| e.into())
    }

    pub fn reset(&mut self){
        self.client.reset();
        self.code = 200;
        self.status = "OK".to_owned();
        self.headers.clear();
        self.sent_head = false;
        self.closed = false;
    }

    pub fn websocket_direct(self) -> WebSocket<BufReader<R>, W> {
        WebSocket::with_split(self.netr, self.netw)
    }
    pub async fn websocket_with_key(mut self, mut wskey: Vec<u8>) -> LibResult<WebSocket<BufReader<R>, W>> {
        wskey.extend_from_slice(MAGIC);

        let reskey = Sha1::digest(&wskey);
        let reskey = b64std.encode(reskey);

        self.code = 101;
        self.status = "Switching Protocols".to_owned();
        self.set_header("Connection", "Upgrade");
        self.set_header("Upgrade", "websocket");
        self.set_header("Sec-WebSocket-Accept", &reskey);
        self.close(b"").await?;

        Ok(self.websocket_direct())
    }
    pub async fn websocket(self) -> LibResult<WebSocket<BufReader<R>, W>> {
        let key = self.client.headers.get("sec-websocket-key").map_or_else(
            || Err(io::Error::new(io::ErrorKind::Other, "missing ws key")), 
            |k| Ok(k[0].as_bytes().to_vec())
        )?;
        self.websocket_with_key(key).await
    }

    pub fn http2_direct(self, settings: Http2Settings) -> Http2Session<BufReader<R>, W> {
        Http2Session::with(self.netr, self.netw, crate::http2::session::Mode::Server, true, settings)
    }
    pub async fn http2_prior_knowledge(mut self) -> LibResult<Http2Session<BufReader<R>, W>> {
        let mut fin = [0; 6];
        if self.client.method == HttpMethod::Unknown(Some("PRI".to_owned())) && self.client.path == "*" && self.client.version == HttpVersion::Unknown(Some("HTTP/2.0".to_owned())) && self.client.headers.len() == 0 {
            self.netr.read_exact(&mut fin).await?;

            if &fin == b"SM\r\n\r\n" {
                Ok(self.http2_direct(Http2Settings::default_no_push()))
            }
            else {
                Err(LibError::InvalidUpgrade)
            }
        }
        else {
            Err(LibError::InvalidUpgrade)
        }
    }
    pub async fn h2c(mut self, settings: Option<Http2Settings>) -> LibResult<Http2Session<BufReader<R>, W>> {
        self.read_until_complete().await?;

        self.code = 101;
        self.status = "Switching Protocols".to_owned();
        
        if let Some(settings) = settings {
            self.set_header("Connection", "Upgrade, HTTP2-Settings");
            self.set_header("Upgrade", "h2c");
            self.set_header("HTTP2-Settings", &b64std.encode(settings.to_vec()));
        }
        else {
            self.set_header("Upgrade", "h2c");
            self.set_header("Connection", "Upgrade");
        }
        
        self.close(b"").await?;

        let settings = 
        if let Some(settings) = self.client.headers.get("http2-settings"){
            let buff = b64std.decode(&settings[0]).unwrap_or(vec![]);
            Http2Settings::from(&buff)
        }
        else {
            Http2Settings::default_no_push()
        };

        let client = self.client;
        let h2 = Http2Session::with(self.netr, self.netw, crate::http2::session::Mode::Server, true, settings);
        let mut curr = Http2Data::empty(1, settings);

        curr.end_head = true;
        curr.end_body = true;
        curr.body = client.body;
        
        for (header, vs) in client.headers {
            for value in vs {
                curr.headers.push((header.as_bytes().to_vec(), value.into_bytes()))
            }
        }

        h2.streams.insert(1, curr);

        Ok(h2)
    }
}

impl<R: ReadStream, W: WriteStream> HttpSocket for Http1Socket<R, W>{
    #[inline]
    fn get_type(&self) -> HttpType {
        HttpType::Http1
    }

    #[inline]
    fn get_client(&self) -> &HttpClient {
        &self.client
    }
    #[inline]
    fn read_client(&'_ mut self) -> impl Future<Output = Result<&'_ HttpClient, LibError>> + Send + '_ {
        self.read_client()
    }
    #[inline]
    fn read_until_complete(&'_ mut self) -> impl Future<Output = Result<&'_ HttpClient, LibError>> + Send + '_ {
        self.read_until_complete()
    }
    #[inline]
    fn read_until_head_complete(&'_ mut self) -> impl Future<Output = Result<&'_ HttpClient, LibError>> + Send + '_ {
        self.read_until_head_complete()
    }

    #[inline] fn add_header(&mut self, header: &str, value: &str) { self.add_header(header, value) }
    #[inline] fn set_header(&mut self, header: &str, value: &str){ self.set_header(header, value) }
    #[inline] fn del_header(&mut self, header: &str) -> Option<Vec<String>>{ self.del_header(header) }

    #[inline]
    fn set_status(&mut self, code: u16, message: String) {
        self.code = code;
        self.status = message;
    }
    #[inline]
    fn write<'a>(&'a mut self, body: &'a [u8]) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.write(body)
    }
    #[inline]
    fn close<'a>(&'a mut self, body: &'a [u8]) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.close(body)
    }
    #[inline]
    fn flush<'a>(&'a mut self) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.flush()
    }
}

