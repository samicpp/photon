pub mod tests;
pub mod frames;
pub mod packets;
pub mod listener;
pub mod session;

// https://datatracker.ietf.org/doc/html/rfc9000
// https://datatracker.ietf.org/doc/html/rfc9221


pub fn read_varint(buf: &[u8], index: &mut usize) -> u64 {
    if buf[*index] & 0xc0 == 0 { 
        *index += 1;
        buf[*index - 1] as u64
    } else if buf[*index] & 0xc0 == 0x40 {
        *index += 2;
        u16::from_be_bytes([
            buf[*index - 2] & 0x3f, 
            buf[*index - 1]
        ]) as u64
    } else if buf[*index] & 0xc0 == 0x80 {
        *index += 4;
        u32::from_be_bytes([
            buf[*index - 4] & 0x3f, 
            buf[*index - 3], 
            buf[*index - 2], 
            buf[*index - 1]
        ]) as u64
    } else {
        *index += 8;
        u64::from_be_bytes([
            buf[*index - 8] & 0x3f,
            buf[*index - 7], 
            buf[*index - 6], 
            buf[*index - 5], 
            buf[*index - 4], 
            buf[*index - 3], 
            buf[*index - 2], 
            buf[*index - 1]
        ])
    }
}
pub fn write_varint(buf: &mut [u8], index: &mut usize, int: u64) {
    #[cfg(debug_assertions)] 
    if int > 0x3fff_ffff_ffff_ffff {
        panic!("varint exceeds maximum encodable 4611686018427387903");
    }

    if int <= 0x3f {
        buf[*index] = int as u8; *index += 1;
    } else if int <= 0x3fff {
        let b = u16::to_be_bytes(int as u16);
        buf[*index] = (b[0] | 0x40) as u8; *index += 1;
        buf[*index] = b[1] as u8; *index += 1;
    } else if int <= 0x3fff_ffff {
        let b = u32::to_be_bytes(int as u32);
        buf[*index] = (b[0] | 0x80) as u8; *index += 1;
        buf[*index] = b[1] as u8; *index += 1;
        buf[*index] = b[2] as u8; *index += 1;
        buf[*index] = b[3] as u8; *index += 1;
    } else {
        let b = u64::to_be_bytes(int);
        buf[*index] = (b[0] | 0xc0) as u8; *index += 1;
        buf[*index] = b[1] as u8; *index += 1;
        buf[*index] = b[2] as u8; *index += 1;
        buf[*index] = b[3] as u8; *index += 1;
        buf[*index] = b[4] as u8; *index += 1;
        buf[*index] = b[5] as u8; *index += 1;
        buf[*index] = b[6] as u8; *index += 1;
        buf[*index] = b[7] as u8; *index += 1;
    }
}

pub fn try_read_varint(buf: &[u8], index: &mut usize) -> Option<u64> {
    if *index >= buf.len() {
        None
    } else if buf[*index] & 0xc0 == 0 { 
        *index += 1;
        Some(buf[*index - 1] as u64)
    } 

    else if *index + 1 >= buf.len() {
        None
    } else if buf[*index] & 0xc0 == 0x40 {
        *index += 2;
        Some(u16::from_be_bytes([
            buf[*index - 2] & 0x3f, 
            buf[*index - 1]
        ]) as u64)
    }

    else if *index + 3 >= buf.len() {
        None
    } else if buf[*index] & 0xc0 == 0x80 {
        *index += 4;
        Some(u32::from_be_bytes([
            buf[*index - 4] & 0x3f, 
            buf[*index - 3], 
            buf[*index - 2], 
            buf[*index - 1]
        ]) as u64)
    } 

    else if *index + 7 >= buf.len() {
        None
    } else {
        *index += 8;
        Some(u64::from_be_bytes([
            buf[*index - 8] & 0x3f,
            buf[*index - 7], 
            buf[*index - 6], 
            buf[*index - 5], 
            buf[*index - 4], 
            buf[*index - 3], 
            buf[*index - 2], 
            buf[*index - 1]
        ]))
    }
}
