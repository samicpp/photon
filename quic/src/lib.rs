use std::array::TryFromSliceError;

use aes::Aes128;
use cipher::{Array, BlockCipherEncrypt, KeyInit};

// #[cfg(test)]
pub mod tests;
pub mod frames;
pub mod packets;
pub mod listener;
pub mod session;
pub mod keys;

// https://datatracker.ietf.org/doc/html/rfc9000
// https://datatracker.ietf.org/doc/html/rfc9221


pub const fn read_varint(buf: &[u8], index: &mut usize) -> u64 {
    let l = (buf[*index] & 0xc0) >> 6;
    match l {
        0b00 => {
            *index += 1;
            buf[*index - 1] as u64
        },
    
        0b01 => {
            *index += 2;
            u16::from_be_bytes([
                buf[*index - 2] & 0x3f, 
                buf[*index - 1]
            ]) as u64
        },
        0b10 => {
            *index += 4;
            u32::from_be_bytes([
                buf[*index - 4] & 0x3f, 
                buf[*index - 3], 
                buf[*index - 2], 
                buf[*index - 1]
            ]) as u64
        },
        _ => { // 0b11
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
}
pub const fn write_varint(buf: &mut [u8], index: &mut usize, int: u64) -> u8 {
    #[cfg(debug_assertions)] 
    if int > 0x3fff_ffff_ffff_ffff {
        panic!("varint exceeds maximum encodable 4611686018427387903");
    }

    if int <= 0x3f {
        buf[*index] = int as u8; *index += 1;
        1
    } else if int <= 0x3fff {
        let b = u16::to_be_bytes(int as u16);
        buf[*index] = (b[0] | 0x40) as u8; *index += 1;
        buf[*index] = b[1] as u8; *index += 1;
        2
    } else if int <= 0x3fff_ffff {
        let b = u32::to_be_bytes(int as u32);
        buf[*index] = (b[0] | 0x80) as u8; *index += 1;
        buf[*index] = b[1] as u8; *index += 1;
        buf[*index] = b[2] as u8; *index += 1;
        buf[*index] = b[3] as u8; *index += 1;
        4
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
        8
    }
}
pub const fn try_read_varint(buf: &[u8], index: &mut usize) -> Option<u64> {
    let l = (buf[*index] & 0xc0) >> 6;
    if buf.len() - *index > 2_usize.pow(l as u32) {
        Some(match l {
            0b00 => {
                *index += 1;
                buf[*index - 1] as u64
            },
        
            0b01 => {
                *index += 2;
                u16::from_be_bytes([
                    buf[*index - 2] & 0x3f, 
                    buf[*index - 1]
                ]) as u64
            },
            0b10 => {
                *index += 4;
                u32::from_be_bytes([
                    buf[*index - 4] & 0x3f, 
                    buf[*index - 3], 
                    buf[*index - 2], 
                    buf[*index - 1]
                ]) as u64
            },
            _ => { // 0b11
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
        })
    }
    else {
        None
    }
}

// appendix A.2 https://datatracker.ietf.org/doc/html/rfc9000#name-sample-packet-number-encodi
pub const fn encode_pn(full_pn: i64, largest_acked: i64) -> u32 {
    let num_acked = 
    if largest_acked == 0 { full_pn + 1 }
    else { full_pn - largest_acked };

    // let min_bits = (num_acked as f64).log2() + 1.0;
    // let num_bytes = (min_bits / 8.0).ceil();
    let num_bytes = 
    if num_acked <= 128 { 1 }
    else if num_acked <= 32_768 { 2 }
    else if num_acked <= 8_388_608 { 3 }
    else { 4 };

    let mask = (1i64 << (num_bytes * 8)) - 1;

    (full_pn & mask) as u32
}
pub const fn decode_pn(largest_pn: i64, truncated_pn: u32, pn_bits: u8) -> i64 {
    let expected = largest_pn + 1;
    let pn_window = 1 << pn_bits * 8;
    let pn_hwin = pn_window / 2;
    let pn_mask = pn_window - 1;
    
    let candidate = expected & !pn_mask | truncated_pn as i64;

    if candidate <= expected - pn_hwin && candidate < 0x4000_0000_0000_0000 - pn_window {
        candidate + pn_window
    } else if candidate > expected + pn_hwin && candidate >= pn_window {
        candidate - pn_window
    } else {
        candidate
    }
}
pub const fn write_pn(buf: &mut [u8], index: &mut usize, full_pn: i64, largest_acked: i64) -> u8 {
    let num_acked = 
    if largest_acked == 0 { full_pn + 1 }
    else { full_pn - largest_acked };

    let num_bytes = 
    if num_acked <= 128 { 1 }
    else if num_acked <= 32_768 { 2 }
    else if num_acked <= 8_388_608 { 3 }
    else { 4 };

    let mask = (1i64 << (num_bytes * 8)) - 1;

    let pn = u32::to_be_bytes((full_pn & mask) as u32);
    let pos = *index;

    match num_bytes {
        1 => { 
            buf[pos] = pn[3]; 
            *index += 1; 
        },
        2 => {
            buf[pos] = pn[2]; 
            buf[pos + 1] = pn[3]; 
            *index += 2;
        },
        3 => {
            buf[pos] = pn[1]; 
            buf[pos + 1] = pn[2]; 
            buf[pos + 2] = pn[3]; 
            *index += 3;
        },
        _ => {
            buf[pos] = pn[0]; 
            buf[pos + 1] = pn[1]; 
            buf[pos + 2] = pn[2]; 
            buf[pos + 3] = pn[3]; 
            *index += 4;
        },
    }
    num_bytes as u8
}

pub fn gen_hp_mask(hp_key: &[u8], sample: &[u8]) -> Result<[u8; 5], TryFromSliceError> {
    let key = cipher::Array::try_from(hp_key)?;
    let cipher = Aes128::new(&key);

    let mut block = Array::try_from(sample)?;
    cipher.encrypt_block(&mut block);
    
    let mut mask = [0; 5];
    mask.copy_from_slice(&block[..5]);
    Ok(mask)
}
