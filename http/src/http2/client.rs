use std::{collections::HashMap, sync::Arc};

use crate::{http2::session::Http2Session, shared::{HttpMethod, HttpRequest, HttpResponse, HttpType, LibError, LibResult, ReadStream, WriteStream, string_from_owned_utf8}};


#[derive(Debug)]
pub struct Http2Request<R: ReadStream, W: WriteStream> {
    pub stream_id: u32,
    pub session: Arc<Http2Session<R, W>>,
    
    pub path: String,
    pub method: HttpMethod,
    pub authority: String,
    pub scheme: String,
    
    pub headers: HashMap<String, Vec<String>>,
    
    pub sent_head: bool,
    pub sent: bool,
    
    pub response: HttpResponse,
    pub is_reset: bool,
}
impl<R: ReadStream, W: WriteStream> Http2Request<R, W> {
    pub fn new(stream_id: u32, session: Arc<Http2Session<R, W>>) -> LibResult<Self> {
        if session.streams.contains_key(&stream_id) {
            Ok(Self {
                stream_id, session,
                path: "/".to_owned(),
                method: HttpMethod::Get,
                authority: String::new(),
                scheme: String::new(),
                headers: HashMap::new(),
                sent_head: false,
                sent: false,
                response: HttpResponse::default(),
                is_reset: false,
            })
        }
        else {
            Err(LibError::InvalidStream)
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

    pub async fn send_head(&mut self, end: bool) -> LibResult<()> {
        if !self.sent_head {
            self.sent_head = true;
            let mut headers = Vec::new();

            headers.push((
                b":method".to_vec(), 
                match &self.method { HttpMethod::Unknown(Some(s)) => s.as_bytes().to_vec(), v => v.to_string().into_bytes()},
            ));
            headers.push((b":scheme".to_vec(), self.scheme.as_bytes().to_vec()));
            headers.push((b":authority".to_vec(), self.authority.as_bytes().to_vec()));
            headers.push((b":path".to_vec(), self.path.as_bytes().to_vec()));

            for (header, values) in self.headers.drain(){
                for value in values {
                    headers.push((header.clone().into_bytes(), value.into_bytes()));
                }
            }

            let head = headers.iter().map(|(h, v)| (h.as_slice(), v.as_slice())).collect::<Vec<(&[u8], &[u8])>>();
            self.session.send_headers(self.stream_id, end, &head).await?;
            Ok(())
        }
        else {
            Err(LibError::HeadersSent)
        }
    }
    pub async fn write(&mut self, buf: &[u8]) -> LibResult<()> {
        if !self.sent_head {
            self.send_head(false).await?;
        }
        self.session.send_data(self.stream_id, false, buf).await
    }
    pub async fn send(&mut self, buf: &[u8]) -> LibResult<()> {
        if !self.sent_head {
            self.set_header("content-length", &buf.len().to_string());
            self.send_head(false).await?;
        }
        self.session.send_data(self.stream_id, true, buf).await
    }

    pub async fn read_response(&mut self) -> LibResult<&HttpResponse> {
        let mut shard = self.session.streams.get_mut(&self.stream_id).unwrap();

        if shard.reset {
            self.is_reset = true;
        }
        else if !self.response.head_complete {
            let mut shard = 
            if !shard.end_head {
                let notif = shard.head_complete.clone();
                drop(shard);
                notif.notified().await;
                self.session.streams.get_mut(&self.stream_id).unwrap()
            }
            else { shard };
            
            self.response.head_complete = true;

            let mut headers = Vec::with_capacity(shard.headers.len());
            headers.append(&mut shard.headers);
            drop(shard);

            for (h, v) in headers {
                let header = string_from_owned_utf8(h);
                let value = string_from_owned_utf8(v);

                if header == ":status" {
                    self.response.code = value.parse().unwrap_or(0);
                }

                else if let Some(values) = self.response.headers.get_mut(&header) {
                    values.push(value)
                }
                else {
                    self.response.headers.insert(header, vec![value]);
                }
            }
        }
        else if !self.response.body_complete {

            let avail = shard.body.len();
            self.response.body.append(&mut shard.body);
            self.response.body_complete = shard.end_body;
            
            if avail == 0 {
                let notif = shard.body_received.clone();
                drop(shard);
                notif.notified().await;
            }
        }

        Ok(&self.response)
    }
    pub async fn read_until_complete(&mut self) -> LibResult<&HttpResponse> {
        while !self.response.body_complete && !self.is_reset {
            self.read_response().await?;
        }
        Ok(&self.response)
    }
    pub async fn read_until_head_complete(&mut self) -> LibResult<&HttpResponse> {
        while !self.response.head_complete && !self.is_reset {
            self.read_response().await?;
        }
        Ok(&self.response)
    }
}
impl<R: ReadStream, W: WriteStream> HttpRequest for Http2Request<R, W> {
    #[inline]
    fn get_type(&self) -> HttpType {
        HttpType::Http2
    }

    #[inline] fn add_header(&mut self, header: &str, value: &str) { self.add_header(&header.to_lowercase(), value) }
    #[inline] fn set_header(&mut self, header: &str, value: &str) { self.set_header(&header.to_lowercase(), value) }
    #[inline] fn del_header(&mut self, header: &str) -> Option<Vec<String>> { self.del_header(&header.to_lowercase()) }
    
    #[inline] fn set_method(&mut self, method: HttpMethod) { self.method = method }
    #[inline] fn set_scheme(&mut self, scheme: String) { self.scheme = scheme }
    #[inline] fn set_path(&mut self, path: String) { self.path = path }
    #[inline] fn set_host(&mut self, host: String) { self.authority = host }

    #[inline]
    fn write<'a>(&'a mut self, body: &'a [u8]) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.write(body)
    }
    #[inline]
    fn send<'a>(&'a mut self, body: &'a [u8]) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.send(body)
    }
    #[inline]
    fn flush<'a>(&'a mut self) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        std::future::ready(Ok(()))
    }

    #[inline]
    fn get_response<'_a>(&'_a self) -> &'_a HttpResponse {
        &self.response
    }
    #[inline]
    fn read_response<'_a>(&'_a mut self) -> impl Future<Output = Result<&'_a HttpResponse, LibError>> + Send + '_a {
        self.read_response()
    }
    #[inline]
    fn read_until_complete<'_a>(&'_a mut self) -> impl Future<Output = Result<&'_a HttpResponse, LibError>> + Send + '_a {
        self.read_until_complete()
    }
    #[inline]
    fn read_until_head_complete<'_a>(&'_a mut self) -> impl Future<Output = Result<&'_a HttpResponse, LibError>> + Send + '_a {
        self.read_until_head_complete()
    }
}
