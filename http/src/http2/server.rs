use std::{collections::HashMap, sync::Arc};

use crate::{http2::session::Http2Session, shared::{HttpClient, HttpSocket, HttpType, LibError, LibResult, ReadStream, WriteStream, string_from_owned_utf8}};


#[derive(Debug)]
pub struct Http2Socket<R: ReadStream, W: WriteStream> {
    pub stream_id: u32,
    pub session: Arc<Http2Session<R, W>>,

    pub client: HttpClient,
    pub is_reset: bool,
    
    pub status: u16,
    pub headers: HashMap<String, Vec<String>>,
    
    pub sent_head: bool,
    pub closed: bool,
}
impl<R: ReadStream, W: WriteStream> Http2Socket<R, W> {
    pub fn new(stream_id: u32, session: Arc<Http2Session<R, W>>) -> LibResult<Self> {
        if session.streams.contains_key(&stream_id) {
            Ok(Self {
                stream_id, session,
                client: HttpClient::default(),
                is_reset: false,
                status: 200,
                headers: HashMap::new(),
                sent_head: false,
                closed: false,
            })
        }
        else {
            Err(LibError::InvalidStream)
        }
    }

    pub async fn read_client(&mut self) -> LibResult<&HttpClient> {
        let mut shard = self.session.streams.get_mut(&self.stream_id).unwrap();

        if shard.reset {
            self.is_reset = true;
        }
        else if !self.client.head_complete {
            let mut shard = 
            if !shard.end_head {
                let notif = shard.head_complete.clone();
                drop(shard);
                notif.notified().await;
                self.session.streams.get_mut(&self.stream_id).unwrap()
            }
            else { shard };
            
            self.client.head_complete = true;

            let mut headers = Vec::with_capacity(shard.headers.len());
            headers.append(&mut shard.headers);
            drop(shard);

            for (h, v) in headers {
                let header = string_from_owned_utf8(h);
                let value = string_from_owned_utf8(v);

                if header == ":method" {
                    self.client.method = value.into();
                }
                else if header == ":scheme" {
                    self.client.scheme = Some(value);
                }
                else if header == ":authority" {
                    self.client.host = Some(value)
                }
                else if header == ":path" {
                    self.client.path = value
                }

                else if let Some(values) = self.client.headers.get_mut(&header) {
                    values.push(value)
                }
                else {
                    self.client.headers.insert(header, vec![value]);
                }
            }
        }
        else if !self.client.body_complete {

            let avail = shard.body.len();
            self.client.body.append(&mut shard.body);
            self.client.body_complete = shard.end_body;
            
            if avail == 0 {
                let notif = shard.body_received.clone();
                drop(shard);
                notif.notified().await;
            }
        }

        Ok(&self.client)
    }
    pub async fn read_until_complete(&mut self) -> LibResult<&HttpClient> {
        while !self.client.body_complete && !self.is_reset {
            self.read_client().await?;
        }
        Ok(&self.client)
    }
    pub async fn read_until_head_complete(&mut self) -> LibResult<&HttpClient> {
        while !self.client.head_complete && !self.is_reset {
            self.read_client().await?;
        }
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

    pub async fn send_head(&mut self, end: bool) -> LibResult<()> {
        if !self.sent_head {
            self.sent_head = true;
            let mut headers = Vec::new();
            headers.push((b":status".to_vec(), self.status.to_string().into_bytes()));

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
    pub async fn close(&mut self, buf: &[u8]) -> LibResult<()> {
        if !self.sent_head {
            self.set_header("content-length", &buf.len().to_string());
            self.send_head(false).await?;
        }
        self.session.send_data(self.stream_id, true, buf).await
    }
}
impl<R: ReadStream, W: WriteStream> HttpSocket for Http2Socket<R, W>{
    #[inline]
    fn get_type(&self) -> HttpType {
        HttpType::Http2
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

    #[inline] fn add_header(&mut self, header: &str, value: &str) { self.add_header(&header.to_lowercase(), value) }
    #[inline] fn set_header(&mut self, header: &str, value: &str){ self.set_header(&header.to_lowercase(), value) }
    #[inline] fn del_header(&mut self, header: &str) -> Option<Vec<String>>{ self.del_header(&header.to_lowercase()) }

    #[inline]
    fn set_status(&mut self, code: u16, _message: String) {
        self.status = code;
    }
    #[inline]
    fn write<'a>(&'a mut self, body: &'a [u8] ) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.write(body)
    }
    #[inline]
    fn close<'a>(&'a mut self, body: &'a [u8] ) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        self.close(body)
    }
    #[inline]
    fn flush<'a>(&'a mut self) -> impl Future<Output = Result<(), LibError>> + Send + 'a {
        std::future::ready(Ok(()))
    }
}