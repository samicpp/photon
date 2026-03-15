use tokio::io::BufReader;

use crate::{http1::{client::Http1Request, server::Http1Socket}, http2::{client::Http2Request, server::Http2Socket}, shared::{HttpClient, HttpMethod, HttpRequest, HttpResponse, HttpSocket, HttpType, LibError, ReadStream, WriteStream}};

pub enum PolyHttpSocket<R: ReadStream, W: WriteStream>{
    Http1(Http1Socket<R, W>),
    Http2(Http2Socket<BufReader<R>, W>),
}

impl<R: ReadStream, W: WriteStream> HttpSocket for PolyHttpSocket<R, W>{
    fn get_type(&self) -> HttpType {
        match self {
            Self::Http1(_) => HttpType::Http1,
            Self::Http2(_) => HttpType::Http2,
        }
    }

    fn get_client(&self) -> &HttpClient {
        match self{
            Self::Http1(h) => &h.client,
            Self::Http2(h) => &h.client,
        }
    }
    async fn read_client(&'_ mut self) -> Result<&'_ HttpClient, LibError> {
        match self{
            Self::Http1(h) => h.read_client().await,
            Self::Http2(h) => h.read_client().await,
        }
    }
    async fn read_until_complete(&'_ mut self) -> Result<&'_ HttpClient, LibError> {
        match self{
            Self::Http1(h) => h.read_until_complete().await,
            Self::Http2(h) => h.read_until_complete().await,
        }
    }
    async fn read_until_head_complete(&'_ mut self) -> Result<&'_ HttpClient, LibError> {
        match self{
            Self::Http1(h) => h.read_until_head_complete().await,
            Self::Http2(h) => h.read_until_head_complete().await,
        }
    }

    fn add_header(&mut self, header: &str, value: &str) { 
        match self {
            Self::Http1(h) => h.add_header(header, value),
            Self::Http2(h) => h.add_header(header, value),
        }
    }
    fn set_header(&mut self, header: &str, value: &str){ 
        match self {
            Self::Http1(h) => h.set_header(header, value),
            Self::Http2(h) => h.set_header(header, value),
        } 
    }
    fn del_header(&mut self, header: &str) -> Option<Vec<String>>{ 
        match self {
            Self::Http1(h) => h.del_header(header),
            Self::Http2(h) => h.del_header(header),
        }
    }

    fn set_status(&mut self, code: u16, message: String) {
        match self {
            Self::Http1(h) => {
                h.code = code;
                h.status = message;
            },
            Self::Http2(h) => h.status = code,
        }
    }
    async fn write<'a>(&'a mut self, body: &'a [u8]) -> Result<(), LibError> {
        match self{
            Self::Http1(h) => h.write(body).await,
            Self::Http2(h) => h.write(body).await,
        }
    }
    async fn close<'a>(&'a mut self, body: &'a [u8]) -> Result<(), LibError> {
        match self{
            Self::Http1(h) => h.close(body).await,
            Self::Http2(h) => h.close(body).await,
        }
    }
    async fn flush<'a>(&'a mut self) -> Result<(), LibError> {
        match self{
            Self::Http1(h) => h.flush().await,
            Self::Http2(h) => h.flush().await,
        }
    }
}

impl<R: ReadStream, W: WriteStream> From<Http1Socket<R, W>> for PolyHttpSocket<R, W> {
    fn from(value: Http1Socket<R, W>) -> Self {
        Self::Http1(value)
    }
}


pub enum PolyHttpRequest<R: ReadStream, W: WriteStream>{
    Http1(Http1Request<R, W>),
    Http2(Http2Request<BufReader<R>, W>),
}

impl<R: ReadStream, W: WriteStream> HttpRequest for PolyHttpRequest<R, W>{
    fn get_type(&self) -> HttpType {
        match self {
            Self::Http1(_) => HttpType::Http1,
            Self::Http2(_) => HttpType::Http2,
        }
    }

    fn add_header(&mut self, header: &str, value: &str) { 
        match self {
            Self::Http1(h) => h.add_header(header, value),
            Self::Http2(h) => h.add_header(header, value),
        }
    }
    fn set_header(&mut self, header: &str, value: &str){ 
        match self {
            Self::Http1(h) => h.set_header(header, value),
            Self::Http2(h) => h.set_header(header, value),
        } 
    }
    fn del_header(&mut self, header: &str) -> Option<Vec<String>>{ 
        match self {
            Self::Http1(h) => h.del_header(header),
            Self::Http2(h) => h.del_header(header),
        }
    }
    
    fn set_method(&mut self, method: HttpMethod){
        match self {
            Self::Http1(h) => h.set_method(method),
            Self::Http2(h) => h.set_method(method),
        }
    }
    fn set_scheme(&mut self, scheme: String) {
        match self {
            Self::Http1(h) => h.set_scheme(scheme),
            Self::Http2(h) => h.set_scheme(scheme),
        }
    }
    fn set_path(&mut self, method: String){
        match self {
            Self::Http1(h) => h.set_path(method),
            Self::Http2(h) => h.set_path(method),
        }
    }
    fn set_host(&mut self, host: String) {
        match self {
            Self::Http1(h) => h.set_host(host),
            Self::Http2(h) => h.set_host(host),
        }
    }

    async fn write<'a>(&'a mut self, body: &'a [u8]) -> Result<(), LibError> {
        match self {
            Self::Http1(h) => h.write(body).await,
            Self::Http2(h) => h.write(body).await,
        }
    }
    async fn send<'a>(&'a mut self, body: &'a [u8]) -> Result<(), LibError> {
        match self {
            Self::Http1(h) => h.send(body).await,
            Self::Http2(h) => h.send(body).await,
        }
    }
    async fn flush<'a>(&'a mut self) -> Result<(), LibError> {
        match self {
            Self::Http1(h) => h.flush().await,
            Self::Http2(h) => h.flush().await,
        }
    }

    fn get_response<'_a>(&'_a self) -> &'_a HttpResponse {
        match self {
            Self::Http1(h) => h.get_response(),
            Self::Http2(h) => h.get_response(),
        }
    }
    async fn read_response<'_a>(&'_a mut self) -> Result<&'_a HttpResponse, LibError> {
        match self {
            Self::Http1(h) => h.read_response().await,
            Self::Http2(h) => h.read_response().await,
        }
    }
    async fn read_until_complete<'_a>(&'_a mut self) -> Result<&'_a HttpResponse, LibError> {
        match self {
            Self::Http1(h) => h.read_until_complete().await,
            Self::Http2(h) => h.read_until_complete().await,
        }
    }
    async fn read_until_head_complete<'_a>(&'_a mut self) -> Result<&'_a HttpResponse, LibError> {
        match self {
            Self::Http1(h) => h.read_until_head_complete().await,
            Self::Http2(h) => h.read_until_head_complete().await,
        }
    }
}

impl<R: ReadStream, W: WriteStream> From<Http1Request<R, W>> for PolyHttpRequest<R, W>{
    fn from(value: Http1Request<R, W>) -> Self {
        Self::Http1(value)
    }
}
