use std::io;

use bytes::BytesMut;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use tokio::net::UdpSocket;

use crate::read_varint;


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


    pub async fn listen(&self, size: usize) -> io::Result<()> {
        let mut buff = BytesMut::new();
        buff.resize(size, 0);

        loop {
            let (len, addr) = self.sock.recv_from(&mut buff).await?;
            println!("{addr:?} -> {len}");
            

            let mut pos = 0;

            while pos < len {

                if buff[pos] & 0x80 != 0 { // long header
                    let mut off = 0;

                    let ty = (buff[pos] & 0b0011_0000) >> 4;
                    let pnl = buff[pos] & 0b0000_0011; // unused in vn and retry, obfuscated
                    let version = u32::from_be_bytes([buff[pos + 1], buff[pos + 2], buff[pos + 3], buff[pos + 4]]);

                    off = 5;
                    
                    let dcidl = buff[pos + off] as usize;
                    let dcid = &buff[pos + off + 1..pos + off + 1 + dcidl];
                    let scidl = buff[pos + off + 1 + dcidl] as usize;
                    let scid = &buff[pos + off + dcidl + 2..pos + off + dcidl + 2 + scidl];

                    off = pos + off + dcidl + 2 + scidl;

                    if version == 0 {
                        // invalid, clients not allowed to send vn
                        break
                    }
                    else {
                        match ty {
                            0 => { // initial
                                let toklen = read_varint(&buff, &mut off) as usize;
                                let token = &buff[off + toklen];
                                off += toklen;

                                let length = read_varint(&buff, &mut off) as usize;

                                println!("\x1b[36mInitial\x1b[0m {scid:?} -> {dcid:?}");

                                pos += off + length; 
                            },
                            1 => { // 0rtt
                                let length = read_varint(&buff, &mut off) as usize;
                                
                                println!("\x1b[36mZeroRTT\x1b[0m {scid:?} -> {dcid:?}");

                                
                                pos += off + length; 
                            }, 
                            2 => { // handshake
                                let length = read_varint(&buff, &mut off) as usize;

                                println!("\x1b[36mZeroRTT\x1b[0m {scid:?} -> {dcid:?}");

                                
                                pos += off + length; 
                            }, 
                            3 => { // retry
                                let rettok = &buff[off..len - 16];
                                let inttag = &buff[len - 16..len];

                                println!("\x1b[36mZeroRTT\x1b[0m {scid:?} -> {dcid:?}");

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

        Ok(())
    }
}