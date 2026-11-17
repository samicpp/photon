use aes_gcm::AeadInOut;
use bytes::BytesMut;
use cipher::{Array, BlockCipherEncrypt};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use tokio::net::UdpSocket;

use crate::{decode_pn, keys::derive_keys, read_varint};


pub struct QuicListener {
    pub sock: UdpSocket, // TODO: change to trait
}

impl QuicListener {
    pub fn bind(address: &SockAddr) -> std::io::Result<Self> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_nonblocking(true)?;
        
        #[cfg(unix)]
        socket.set_reuse_port(true)?;
        socket.set_reuse_address(true)?; 
        
        socket.bind(address)?;

        let sock = UdpSocket::from_std(socket.into())?;

        Ok(Self::with(sock))
    }
    pub fn with(sock: UdpSocket) -> Self {
        Self {
            sock,
        }
    }


    pub async fn listen(&self, size: usize) -> std::io::Result<()> {
        let mut buff = BytesMut::new();
        buff.resize(size, 0);

        loop {
            let (len, addr) = self.sock.recv_from(&mut buff).await?;
            println!("{addr:?} -> {len}");
            

            let mut pos = 0;

            while pos < len {

                if buff[pos] & 0x80 != 0 { // long header
                    let mut off = pos;

                    let ty = (buff[pos] & 0b0011_0000) >> 4;
                    let pnl = buff[pos] & 0b0000_0011; // unused in vn and retry, obfuscated
                    let version = u32::from_be_bytes([buff[pos + 1], buff[pos + 2], buff[pos + 3], buff[pos + 4]]);

                    off += 5;
                    
                    let dcidl = buff[off] as usize;
                    let dcid = buff[off + 1..off + 1 + dcidl].to_vec();
                    let scidl = buff[off + 1 + dcidl] as usize;
                    let scid = buff[off + dcidl + 2..off + dcidl + 2 + scidl].to_vec();

                    off = off + dcidl + 2 + scidl;

                    if version == 0 {
                        // invalid, clients not allowed to send vn
                        break
                    }
                    else {
                        match ty {
                            0 => { // initial
                                let Some(keys) = derive_keys(&dcid) else { continue }; // invalid if None
                                
                                let toklen = read_varint(&buff, &mut off) as usize;
                                let token = &buff[off..off + toklen];
                                off += toklen;
                                
                                let length = read_varint(&buff, &mut off) as usize;
                                
                                let hp_sample = &buff[off + 4..off + 20];
                                
                                let Ok(mut hp_mask) = Array::try_from(hp_sample) else { continue };
                                keys.client_hp_cipher.encrypt_block(&mut hp_mask);
                                
                                buff[pos] ^= hp_mask[0] & 0b0000_1111;
                                let dpnl = buff[pos] & 0b0000_0011;

                                for i in 0..dpnl as usize + 1 {
                                    buff[off + i] ^= hp_mask[i + 1];
                                }

                                let pn = match dpnl {
                                    0 => u32::from_be_bytes([0, 0, 0, buff[off]]),
                                    1 => u32::from_be_bytes([0, 0, buff[off], buff[off + 1]]),
                                    2 => u32::from_be_bytes([0, buff[off], buff[off + 1], buff[off + 2]]),
                                    _ => u32::from_be_bytes([buff[off], buff[off + 1], buff[off + 2], buff[off + 3]]),
                                    // _ => unreachable!("4 bits cant total > 3") // impossible outcome
                                };

                                let real_pn = decode_pn(0, pn, dpnl + 1);
                                // off += 1 + dpnl as usize;
                                
                                println!("\x1b[36mInitial\x1b[0m {dpnl} {real_pn} {scid:?} -> {dcid:?}");

                                // header and payload decryption

                                let mut nonce = [0; 12];
                                nonce.copy_from_slice(&keys.client_iv);
                                let pn_be = real_pn.to_be_bytes();
                                for i in 0..8 {
                                    nonce[i + 4] ^= pn_be[i];
                                }
                                let Ok(nonce) = nonce.as_ref().try_into() else { continue };

                                {
                                    // let payload_range = off + 1 + dpnl as usize..off + length;
                                    // let split_at = off + length - 16 - pos;

                                    let (aad_msg, tag) = (&mut buff[pos..off + length]).split_at_mut(off + length - 16 - pos);
                                    let (aad, msg) = aad_msg.split_at_mut(off + 1 + dpnl as usize - pos);
                                    let Ok(tag) = (&(*tag)).try_into() else { continue };

                                    let Ok(_) = keys.client_key_cipher.decrypt_inout_detached(
                                        nonce, 
                                        aad, 
                                        msg.into(),
                                        tag,
                                    ) else { continue };
                                }

                                let string = String::from_utf8_lossy(&buff[off + 1 + dpnl as usize..off + length]);
                                println!("{string}");

                                // println!("ty = {ty}, pnl = {pnl}, version = {version}");
                                // println!("scid = {scid}, dcid = {dcid}");
                                // println!("dpnl = {dpnl}, pn = {pn}, real_pn = {real_pn}");
                                // dbg!(keys);


                                pos = off + length; 
                            },
                            1 => { // 0rtt
                                let length = read_varint(&buff, &mut off) as usize;
                                
                                println!("\x1b[36mZeroRTT\x1b[0m {scid:?} -> {dcid:?}");

                                
                                pos = off + length; 
                            }, 
                            2 => { // handshake
                                let length = read_varint(&buff, &mut off) as usize;

                                println!("\x1b[36mHandsnake\x1b[0m {scid:?} -> {dcid:?}");

                                
                                pos = off + length; 
                            }, 
                            3 => { // retry
                                let rettok = &buff[off..len - 16];
                                let inttag = &buff[len - 16..len];

                                println!("\x1b[36mRetry\x1b[0m {scid:?} -> {dcid:?}");

                                pos = len;
                            }, 

                            _ => unreachable!(),
                        }
                    }
                }
                else { // 1rtt

                    pos = len;
                }

            }
        }

        #[allow(unreachable_code)] Ok(())
    }
}