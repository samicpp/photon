use http::shared::LibError;

pub const NO_ERR: i32 = -1;
pub const TYPE_ERR: i32 = 1;
pub const ERROR: i32 = 0x100;
pub const IO_ERROR: i32 = 0x200;


pub trait Errno {
    fn get_errno(&self) -> i32;
}

impl Errno for LibError {
    fn get_errno(&self) -> i32 {
        match self {
            Self::Io(io) => io.get_errno(),
            Self::Huffman(_) => 0x101,
            Self::Hpack(_) => 0x102,

            Self::NotConnected => 0x103,
            Self::ConnectionClosed => 0x104,
            Self::StreamClosed => 0x105,
            Self::HeadersSent => 0x106,

            Self::Invalid => 0x107,
            Self::InvalidFrame => 0x108,
            Self::InvalidUpgrade => 0x109,
            Self::InvalidStream => 0x110,
            Self::InvalidString => 0x111,

            Self::NotAccepted => 0x112,
            Self::ResetStream => 0x113,
            Self::Goaway => 0x114,
            Self::ProtocolError => 0x115,
        }
    }
}
impl Errno for std::io::Error {
    fn get_errno(&self) -> i32 {
        self.raw_os_error().unwrap_or(0) | IO_ERROR
    }
}
