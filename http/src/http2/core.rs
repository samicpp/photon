use std::ops::Range;

use tokio::io::AsyncReadExt;

use crate::shared::ReadStream;


#[derive(Debug, Clone)]
pub struct Http2Frame{
    pub source: Vec<u8>,

    pub length: u32,
    pub type_byte: u8,
    pub flags: u8,
    pub stream_id: u32,
    pub pad_len: u8,

    pub priority: Range<usize>,
    pub payload: Range<usize>,
    pub padding: Range<usize>,

    pub ftype: Http2FrameType,
}
impl Http2Frame{
    pub fn from_owned(source: Vec<u8>) -> Option<Self> {
        let length = u32::from_be_bytes([0, *source.get(0)?, *source.get(1)?, *source.get(2)?]);
        let type_byte = *source.get(3)?;
        let flags = *source.get(4)?;
        let stream_id = u32::from_be_bytes([*source.get(5)?, *source.get(6)?, *source.get(7)?, *source.get(8)?]);
        let ftype = type_byte.into();

        let mut pad_len = 0;
        let mut pay_start = 9;
        let mut pay_end = length as usize + 9;

        if flags & 0x08 != 0 {
            pad_len = *source.get(pay_start)?;
            pay_start += 1;
            pay_end -= pad_len as usize;
        }
        if flags & 0x20 != 0 {
            pay_start += 5;
        }

        let priority = if flags & 0x08 != 0 { 10 } else { 9 }..pay_start;
        let payload = pay_start..pay_end;
        let padding = pay_end..pay_end + pad_len as usize;

        Some(Self {
            source,
            length,
            type_byte,
            flags,
            stream_id,
            pad_len,

            priority,
            payload,
            padding,

            ftype,
        })
    }
    pub async fn from_reader<R: ReadStream>(stream: &mut R) -> Result<Self, std::io::Error> {
        let mut source = vec![0; 9];
        stream.read_exact(&mut source).await?;

        let length = ((source[0] as u32) << 16) | ((source[1] as u32) << 8) | source[2] as u32;
        let type_byte = source[3];
        let flags = source[4];
        let stream_id = ((source[5] as u32) << 24) | ((source[6] as u32) << 16) | ((source[7] as u32) << 8) | source[8] as u32;
        let ftype = type_byte.into();

        let mut pad_len = 0;
        let mut pay_start = 9;
        let mut pay_end = length as usize + 9;

        source.resize(9 + length as usize, 0);
        stream.read_exact(&mut source[9..]).await?;
        

        if flags & 0x08 != 0 {
            pad_len = source[pay_start];
            pay_start += 1;
            pay_end -= pad_len as usize;
        }
        if flags & 0x20 != 0 {
            pay_start += 5;
        }

        let priority = if flags & 0x08 != 0 { 10 } else { 9 }..pay_start;
        let payload = pay_start..pay_end;
        let padding = pay_end..pay_end + pad_len as usize;


        Ok(Self {
            source,
            length,
            type_byte,
            flags,
            stream_id,
            pad_len,

            priority,
            payload,
            padding,

            ftype,
        })
    }

    pub fn create(ftype: impl Into<u8>, flags: u8, stream_id: u32, priority: Option<&[u8]>, payload: Option<&[u8]>, padding: Option<&[u8]>) -> Vec<u8> {
        let mut priority = priority.filter(|s| s.len() == 5);
        let mut payload = payload.filter(|s| s.len() < 16777216);
        let mut padding = padding.filter(|s| s.len() < 256);

        let length = 
            priority.and_then(|s| Some(s.len())).unwrap_or(0) + 
            payload.and_then(|s| Some(s.len())).unwrap_or(0) + 
            padding.and_then(|s| Some(s.len() + 1)).unwrap_or(0);

        let length =         
        if length > 16777216 {
            priority = None;
            payload = None;
            padding = None;
            0
        }
        else {
            length
        };
        
        let mut frame = vec![0; 9 + length];

        frame[0] = ((length & 0xff0000) >> 16) as u8;
        frame[1] = ((length & 0x00ff00) >> 8) as u8;
        frame[2] = length as u8;

        frame[3] = ftype.into();
        frame[4] = flags |
        if priority.is_some() { 0x20 } else { 0x00 } |
        if padding. is_some() { 0x08 } else { 0x00 } ;

        frame[5..9].copy_from_slice(&u32::to_be_bytes(stream_id));

        let mut start = 9;

        if let Some(pad) = padding {
            frame[start] = pad.len() as u8;
            start += 1;
        }
        if let Some(priority) = priority {
            frame[start..start + 5].copy_from_slice(priority);
            start += 5;
        }
        if let Some(payload) = payload {
            frame[start..start + payload.len()].copy_from_slice(payload);
        }
        if let Some(padding) = padding {
            let off = frame.len() - padding.len();
            frame[off..].copy_from_slice(padding);
        }

        frame
    }


    pub fn get_priority(&self) -> &[u8] {
        &self.source[self.priority.clone()]
    }
    pub fn get_payload(&self) -> &[u8] {
        &self.source[self.payload.clone()]
    }
    pub fn get_padding(&self) -> &[u8] {
        &self.source[self.padding.clone()]
    }

    pub fn is_ack(&self) -> bool { self.flags & 0x01 != 0 }
    pub fn is_end_stream(&self) -> bool { self.flags & 0x01 != 0 }
    pub fn is_end_headers(&self) -> bool { self.flags & 0x04 != 0 }
    pub fn is_padded(&self) -> bool { self.flags & 0x08 != 0 }
    pub fn is_priority(&self) -> bool { self.flags & 0x20 != 0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Http2FrameType{
    Data,
    Headers,
    Priority,
    RstStream,
    Settings,
    PushPromise,
    Ping,
    Goaway,
    WindowUpdate,
    Continuation,
    
    Invalid(u8),
}
impl From<u8> for Http2FrameType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Data,
            1 => Self::Headers,
            2 => Self::Priority,
            3 => Self::RstStream,
            4 => Self::Settings,
            5 => Self::PushPromise,
            6 => Self::Ping,
            7 => Self::Goaway,
            8 => Self::WindowUpdate,
            9 => Self::Continuation,

            v => Self::Invalid(v),
        }
    }
}
impl Into<u8> for Http2FrameType {
    fn into(self) -> u8 {
        match self {
            Self::Data => 0,
            Self::Headers => 1,
            Self::Priority => 2,
            Self::RstStream => 3,
            Self::Settings => 4,
            Self::PushPromise => 5,
            Self::Ping => 6,
            Self::Goaway => 7,
            Self::WindowUpdate => 8,
            Self::Continuation => 9,

            Self::Invalid(v) => v,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Http2Settings {
    pub header_table_size: Option<u32>,       // 1
    pub enable_push: Option<u32>,             // 2
    pub max_concurrent_streams: Option<u32>,  // 3
    pub initial_window_size: Option<u32>,     // 4
    pub max_frame_size: Option<u32>,          // 5
    pub max_header_list_size: Option<u32>,    // 6
}
impl Http2Settings {
    pub const fn empty() -> Self {
        Self {
            header_table_size: None,
            enable_push: None,
            max_concurrent_streams: None,
            initial_window_size: None,
            max_frame_size: None,
            max_header_list_size: None,
        }
    }
    pub const fn default() -> Self {
        Self {
            header_table_size: Some(4096),
            enable_push: Some(1),
            max_concurrent_streams: None,
            initial_window_size: Some(65535),
            max_frame_size: Some(65535),
            max_header_list_size: None,
        }
    }
    pub const fn default_no_push() -> Self {
        Self {
            header_table_size: Some(4096),
            enable_push: None,
            max_concurrent_streams: None,
            initial_window_size: Some(65535),
            max_frame_size: Some(65535),
            max_header_list_size: None,
        }
    }
    pub const fn maximum() -> Self {
        Self {
            // unsigned, to be safe
            header_table_size: Some(2147483647),
            enable_push: None,
            max_concurrent_streams: Some(2147483647),
            initial_window_size: Some(2147483647),
            max_frame_size: Some(16777215),
            max_header_list_size: None,
        }
    }

    pub fn raw_from(buf: &[u8]) -> Option<Vec<(u16, u32)>> {
        if buf.len() % 6 != 0 { return None }
        
        let mut total = Vec::with_capacity(buf.len() / 6);

        for chunk in buf.chunks_exact(6) {
            let sid = u16::from_be_bytes([chunk[0], chunk[1]]);
            let val = u32::from_be_bytes([chunk[2], chunk[3], chunk[4], chunk[5]]);
            
            total.push((sid, val));
        }

        Some(total)
    }
    pub fn from(buf: &[u8]) -> Self {
        let mut sett = Self::empty();
        let rset = Self::raw_from(buf);

        if let Some(rset) = rset {
            for (id, val) in rset {
                if id == 1 { sett.header_table_size = Some(val) }
                else if id == 2 { sett.enable_push = Some(val) }
                else if id == 3 { sett.max_concurrent_streams = Some(val) }
                else if id == 4 { sett.initial_window_size = Some(val) }
                else if id == 5 { sett.max_frame_size = Some(val) }
                else if id == 6 { sett.max_header_list_size = Some(val) }
            }
        }

        sett
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut res = vec![];

        if let Some(val) = self.header_table_size { res.extend_from_slice(&[0, 1]); res.extend_from_slice(&u32::to_be_bytes(val)); }
        if let Some(val) = self.enable_push { res.extend_from_slice(&[0, 2]); res.extend_from_slice(&u32::to_be_bytes(val)); }
        if let Some(val) = self.max_concurrent_streams { res.extend_from_slice(&[0, 3]); res.extend_from_slice(&u32::to_be_bytes(val)); }
        if let Some(val) = self.initial_window_size { res.extend_from_slice(&[0, 4]); res.extend_from_slice(&u32::to_be_bytes(val)); }
        if let Some(val) = self.max_frame_size { res.extend_from_slice(&[0, 5]); res.extend_from_slice(&u32::to_be_bytes(val)); }
        if let Some(val) = self.max_header_list_size { res.extend_from_slice(&[0, 6]); res.extend_from_slice(&u32::to_be_bytes(val)); }
    
        res
    }
}
impl Default for Http2Settings {
    fn default() -> Self {
        Self::default()
    }
}
