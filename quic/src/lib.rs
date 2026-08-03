use std::{borrow::{Borrow, BorrowMut}, ops::{Deref, DerefMut}};

pub mod tests;
pub mod frames;
pub mod packets;

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



#[derive(Debug)]
pub enum Buffer<const L: usize = 0> { Stack([u8; L]), Heap(Vec<u8>) }

impl<const L: usize> Buffer<L> {
    pub const fn get(&self) -> &[u8] {
        match self {
            Self::Stack(arr) => arr,
            Self::Heap(vec) => vec.as_slice(),
        }
    }
    pub const fn get_mut(&mut self) -> &mut [u8] {
        match self {
            Self::Stack(arr) => arr,
            Self::Heap(vec) => vec.as_mut_slice(),
        }
    }

    pub const fn get_slice(&self, offset: usize, length: usize) -> &[u8] {
        let buf = self.get();
        let (_, tail) = buf.split_at(offset);
        tail.split_at(length).0
    }
    pub const fn get_slice_mut(&mut self, offset: usize, length: usize) -> &mut [u8] {
        let buf = self.get_mut();
        let (_, tail) = buf.split_at_mut(offset);
        tail.split_at_mut(length).0
    }

    pub const fn len(&self) -> usize {
        match self {
            Self::Stack(_) => L,
            Self::Heap(vec) => vec.len(),
        }
    }
}
impl<const L: usize> AsRef<[u8]> for Buffer<L> {
    fn as_ref(&self) -> &[u8] {
        self.get()
    }
}
impl<const L: usize> Deref for Buffer<L> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
impl<const L: usize> DerefMut for Buffer<L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
impl<const L: usize> Borrow<[u8]> for Buffer<L> {
    fn borrow(&self) -> &[u8] {
        self.get()
    }
}
impl<const L: usize> BorrowMut<[u8]> for Buffer<L> {
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.get_mut()
    }
}
impl From<Vec<u8>> for Buffer<0> {
    fn from(value: Vec<u8>) -> Self {
        Self::Heap(value)
    }
}
impl<const L: usize> From<[u8; L]> for Buffer<L> {
    fn from(value: [u8; L]) -> Self {
        Self::Stack(value)
    }
}
impl<const L: usize> Clone for Buffer<L> {
    fn clone(&self) -> Self {
        match self {
            Self::Heap(vec) => Self::Heap(vec.clone()),
            Self::Stack(arr) => Self::Heap(arr.to_vec()),
        }
    }
}


