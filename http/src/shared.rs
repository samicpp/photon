use std::{fmt::Display, collections::HashMap, pin::Pin};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::http2::hpack::{HpackError, huffman::HuffmanError};



pub trait ReadStream: AsyncRead + Unpin + Send + Sync {}
impl<A> ReadStream for A where A: AsyncRead + Unpin + Send + Sync {}

pub trait WriteStream: AsyncWrite + Unpin + Send + Sync {}
impl<A> WriteStream for A where A: AsyncWrite + Unpin + Send + Sync {}

pub trait Stream: ReadStream + WriteStream {}
impl<A> Stream for A where A: ReadStream + WriteStream {}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpType{
    Http1,
    Http2,
    Http3,
}
impl Display for HttpType{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Self::Http1 => write!(f, "Http1"),
            Self::Http2 => write!(f, "Http2"),
            Self::Http3 => write!(f, "Http3"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpVersion{
    Unknown(Option<String>),
    Debug,

    Http09,
    Http10,
    Http11,
    Http2,
    Http3,
}
impl Display for HttpVersion{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(s) => if let Some(v) = s { write!(f, "Unknown({})", v) } else { write!(f, "Unknown") },
            Self::Debug => write!(f, "Debug"),
            
            Self::Http09 => write!(f, "HTTP/0.9"),
            Self::Http10 => write!(f, "HTTP/1.0"),
            Self::Http11 => write!(f, "HTTP/1.1"),
            Self::Http2 => write!(f, "HTTP/2"),
            Self::Http3 => write!(f, "HTTP/3"),
        }
    }
}
impl HttpVersion{
    pub fn is_unknown(&self) -> bool { match self { Self::Unknown(_) => true, _ => false } }
    pub fn is_debug(&self) -> bool { match self { Self::Debug => true, _ => false } }
    pub fn is_http09(&self) -> bool { match self { Self::Http09 => true, _ => false } }
    pub fn is_http10(&self) -> bool { match self { Self::Http10 => true, _ => false } }
    pub fn is_http11(&self) -> bool { match self { Self::Http11 => true, _ => false } }
    pub fn is_http2(&self) -> bool { match self { Self::Http2 => true, _ => false } }
    pub fn is_http3(&self) -> bool { match self { Self::Http3 => true, _ => false } }

    pub fn to_string_unknown(&self) -> String {
        match &self { 
            HttpVersion::Unknown(Some(s)) => s.to_owned(),
            _ => self.to_string()
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HttpMethod{
    Unknown(Option<String>),

    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
}
impl Display for HttpMethod{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(s) => if let Some(v) = s { write!(f, "Unknown({})", v) } else { write!(f, "Unknown") },

            Self::Get => write!(f, "GET"),
            Self::Head => write!(f, "HEAD"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Delete => write!(f, "DELETE"),
            Self::Connect => write!(f, "CONNECT"),
            Self::Options => write!(f, "OPTIONS"),
            Self::Trace => write!(f, "TRACE"),
        }
    }
}
impl HttpMethod{
    pub fn from(string: &str) -> Self{
        if string.eq_ignore_ascii_case("get") { Self::Get }
        else if string.eq_ignore_ascii_case("head") { Self::Head }
        else if string.eq_ignore_ascii_case("post") { Self::Post }
        else if string.eq_ignore_ascii_case("put") { Self::Put }
        else if string.eq_ignore_ascii_case("delete") { Self::Delete }
        else if string.eq_ignore_ascii_case("connect") { Self::Connect }
        else if string.eq_ignore_ascii_case("options") { Self::Options }
        else if string.eq_ignore_ascii_case("trace") { Self::Trace }
        else { Self::Unknown(Some(string.to_owned())) }
    }

    pub fn is_unknown(&self) -> bool { match self { Self::Unknown(_) => true, _ => false } }
    pub fn is_get(&self) -> bool { match self { Self::Get => true, _ => false } }
    pub fn is_head(&self) -> bool { match self { Self::Head => true, _ => false } }
    pub fn is_post(&self) -> bool { match self { Self::Post => true, _ => false } }
    pub fn is_put(&self) -> bool { match self { Self::Put => true, _ => false } }
    pub fn is_delete(&self) -> bool { match self { Self::Delete => true, _ => false } }
    pub fn is_connect(&self) -> bool { match self { Self::Connect => true, _ => false } }
    pub fn is_options(&self) -> bool { match self { Self::Options => true, _ => false } }
    pub fn is_trace(&self) -> bool { match self { Self::Trace => true, _ => false } }
}
impl From<&str> for HttpMethod{
    fn from(value: &str) -> Self {
        Self::from(value)
    }
}
impl From<String> for HttpMethod{
    fn from(value: String) -> Self {
        Self::from(&value)
    }
}
impl From<&String> for HttpMethod{
    fn from(value: &String) -> Self {
        Self::from(value)
    }
}



/*pub trait HttpClient{
    fn is_valid(&self) -> bool;
    fn is_complete(&self) -> (bool, bool);
    
    fn get_method<'_a>(&'_a self) -> &'_a HttpMethod;
    fn get_path<'_a>(&'_a self) -> &'_a str;
    fn get_version<'_a>(&'_a self) -> &'_a HttpVersion;

    fn get_host<'_a>(&'_a self) -> Option<&'_a str>;
    fn get_headers<'_a>(&'_a self) -> &'_a HashMap<String, Vec<String>>;
    fn get_body<'_a>(&'_a self) -> &'_a [u8];

    fn clone(&self) -> Box<dyn HttpClient>;
}*/

#[derive(Debug, Clone)]
pub struct HttpClient{
    pub valid: bool,

    pub mpv_complete: bool,
    pub head_complete: bool,
    pub body_complete: bool,
    
    pub method: HttpMethod,
    pub path: String,
    pub version: HttpVersion,

    pub headers: HashMap<String, Vec<String>>,
    pub body: Vec<u8>,

    pub host: Option<String>, // should not be None in valid clients
    pub scheme: Option<String>,
}
impl HttpClient{
    pub fn reset(&mut self) { *self = Default::default() }
}
impl Default for HttpClient{
    fn default() -> Self {
        Self {
            valid: true,

            mpv_complete: false,
            head_complete: false,
            body_complete: false,

            method: HttpMethod::Unknown(None),
            path: String::new(),
            version: HttpVersion::Unknown(None),

            headers: HashMap::new(),
            body: Vec::new(),

            host: None,
            scheme: None,
        }
    }
}


pub trait HttpSocket{
    fn get_type(&self) -> HttpType;

    fn get_client<'_a>(&'_a self) -> &'_a HttpClient;
    fn read_client<'_a>(&'_a mut self) -> Pin<Box<dyn Future<Output = Result<&'_a HttpClient, LibError>> + Send + '_a>>;
    fn read_until_complete<'_a>(&'_a mut self) -> Pin<Box<dyn Future<Output = Result<&'_a HttpClient, LibError>> + Send + '_a>>;
    fn read_until_head_complete<'_a>(&'_a mut self) -> Pin<Box<dyn Future<Output = Result<&'_a HttpClient, LibError>> + Send + '_a>>;

    fn add_header(&mut self, header: &str, value: &str);
    fn set_header(&mut self, header: &str, value: &str);
    fn del_header(&mut self, header: &str) -> Option<Vec<String>>;
    
    fn set_status(&mut self, code: u16, message: String);
    fn write<'a>(&'a mut self, body: &'a [u8]) -> Pin<Box<dyn Future<Output = Result<(), LibError>> + Send + 'a>>;
    fn close<'a>(&'a mut self, body: &'a [u8]) -> Pin<Box<dyn Future<Output = Result<(), LibError>> + Send + 'a>>;
    fn flush<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), LibError>> + Send + 'a>>;
}

// pub type DynHttpSocket = Box<dyn HttpSocket>;



#[derive(Debug, Clone)]
pub struct HttpResponse{
    pub valid: bool,

    pub vcs_complete: bool,
    pub head_complete: bool,
    pub body_complete: bool,

    pub version: HttpVersion,
    pub code: u16,
    pub status: String,

    pub headers: HashMap<String, Vec<String>>,
    pub body: Vec<u8>,
}
impl Default for HttpResponse{
    fn default() -> Self {
        Self {
            valid: true,

            vcs_complete: false,
            head_complete: false,
            body_complete: false,

            version: HttpVersion::Unknown(None),
            code: 0,
            status: String::new(),

            headers: HashMap::new(),
            body: Vec::new(),
        }
    }
}
impl HttpResponse{
    pub fn reset(&mut self){
        *self = Default::default();
    }
}

pub trait HttpRequest{
    fn get_type(&self) -> HttpType;

    fn add_header(&mut self, header: &str, value: &str);
    fn set_header(&mut self, header: &str, value: &str);
    fn del_header(&mut self, header: &str) -> Option<Vec<String>>;
    
    fn set_method(&mut self, method: HttpMethod);
    fn set_scheme(&mut self, scheme: String);
    fn set_path(&mut self, path: String);
    fn set_host(&mut self, host: String);

    fn write<'a>(&'a mut self, body: &'a [u8]) -> Pin<Box<dyn Future<Output = Result<(), LibError>> + Send + 'a>>;
    fn send<'a>(&'a mut self, body: &'a [u8]) -> Pin<Box<dyn Future<Output = Result<(), LibError>> + Send + 'a>>;
    fn flush<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<(), LibError>> + Send + 'a>>;

    fn get_response<'_a>(&'_a self) -> &'_a HttpResponse;
    fn read_response<'_a>(&'_a mut self) -> Pin<Box<dyn Future<Output = Result<&'_a HttpResponse, LibError>> + Send + '_a>>;
    fn read_until_complete<'_a>(&'_a mut self) -> Pin<Box<dyn Future<Output = Result<&'_a HttpResponse, LibError>> + Send + '_a>>;
    fn read_until_head_complete<'_a>(&'_a mut self) -> Pin<Box<dyn Future<Output = Result<&'_a HttpResponse, LibError>> + Send + '_a>>;
}


#[derive(Debug)]
pub enum LibError {
    Io(std::io::Error),
    Huffman(HuffmanError),
    Hpack(HpackError),
    
    NotConnected,
    ConnectionClosed,
    StreamClosed,
    HeadersSent,

    Invalid,
    InvalidFrame,
    InvalidUpgrade,
    InvalidStream,
    InvalidString,

    NotAccepted,
    ResetStream,
    Goaway,
    ProtocolError,
}
impl LibError {
    pub fn io(&self) -> Option<&std::io::Error> { if let Self::Io(io) = self { Some(io) } else { None } }
    pub fn huffman(&self) -> Option<&HuffmanError> { if let Self::Huffman(err) = self { Some(err) } else { None } }
    pub fn hpack(&self) -> Option<&HpackError> { if let Self::Hpack(err) = self { Some(err) } else { None } }
    
    pub fn is_not_connected(&self) -> bool { if let Self::NotConnected = self { true } else { false } }
    pub fn is_connection_closed(&self) -> bool { if let Self::ConnectionClosed = self { true } else { false } }
    pub fn is_stream_closed(&self) -> bool { if let Self::StreamClosed = self { true } else { false } }
    pub fn is_headers_sent(&self) -> bool { if let Self::HeadersSent = self { true } else { false } }
    
    pub fn is_invalid(&self) -> bool { if let Self::Invalid = self { true } else { false } }
    pub fn is_invalid_frame(&self) -> bool { if let Self::InvalidFrame = self { true } else { false } }
    pub fn is_invalid_upgrade(&self) -> bool { if let Self::InvalidUpgrade = self { true } else { false } }
    pub fn is_invalid_stream(&self) -> bool { if let Self::InvalidStream = self { true } else { false } }
    pub fn is_invalid_string(&self) -> bool { if let Self::InvalidString = self { true } else { false } }
    
    pub fn is_not_accepted(&self) -> bool { if let Self::NotAccepted = self { true } else { false } }
    pub fn is_reset_stream(&self) -> bool { if let Self::ResetStream = self { true } else { false } }
    pub fn is_goaway(&self) -> bool { if let Self::Goaway = self { true } else { false } }
    pub fn is_protocol_error(&self) -> bool { if let Self::ProtocolError = self { true } else { false } }
}
impl From<std::io::Error> for LibError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<HuffmanError> for LibError {
    fn from(value: HuffmanError) -> Self {
        Self::Huffman(value)
    }
}
impl From<HpackError> for LibError {
    fn from(value: HpackError) -> Self {
        Self::Hpack(value)
    }
}
impl Display for LibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{}", err.to_string()),
            Self::Huffman(err) => write!(f, "{}", err.to_string()),
            Self::Hpack(err) => writeln!(f, "{}", err.to_string()),

            Self::NotConnected => writeln!(f, "Not connected to endpoint"),
            Self::ConnectionClosed => writeln!(f, "Connection is closed"),
            Self::StreamClosed => writeln!(f, "Stream is closed"),
            Self::HeadersSent => writeln!(f, "Headers are sent"),

            Self::Invalid => writeln!(f, "Invalid"),
            Self::InvalidFrame => writeln!(f, "Invalid frame"),
            Self::InvalidUpgrade => writeln!(f, "Invalid upgrade"),
            Self::InvalidStream => writeln!(f, "Invalid stream"),
            Self::InvalidString => writeln!(f, "Invalid string"),

            Self::NotAccepted => writeln!(f, "Not accepted"),
            Self::ResetStream => writeln!(f, "stream reset"),
            Self::Goaway => writeln!(f, "Goaway received"),
            Self::ProtocolError => writeln!(f, "Protocol error"),
        }
    }
}
impl std::error::Error for LibError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            Self::Io(e) => Some(e),
            Self::Huffman(e) => Some(e),
            Self::Hpack(e) => Some(e),
            _ => None,
        }
    }
    /*fn description(&self) -> &str {
        self.to_string()
    }*/
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Huffman(e) => Some(e),
            Self::Hpack(e) => Some(e),
            _ => None,
        }
    }
}
pub type LibResult<T> = Result<T, LibError>;


pub(crate) fn string_from_owned_utf8(vec: Vec<u8>) -> String {
    match String::from_utf8(vec) {
        Ok(s) => s,
        Err(e) => {
            let vec = e.into_bytes();
            let cow = String::from_utf8_lossy(&vec);
            cow.into_owned()
        }
    }
}