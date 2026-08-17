#![cfg(test)]

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};

use bytes::BytesMut;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

use crate::{listener::QuicListener, read_varint, write_varint};

#[test]
fn three_is_three(){
    assert!(3 == 3)
}

#[test]
fn varints() {
    let buf = [0x3f, 12, 0x41, 255, 0x80, 0, 255, 255, 0xe0, 0, 0, 0, 0, 0, 0, 1];
    let mut index = 0;
    
    assert_eq!(read_varint(&buf, &mut index), 63);
    assert_eq!(read_varint(&buf, &mut index), 12);
    assert_eq!(read_varint(&buf, &mut index), 511);
    assert_eq!(read_varint(&buf, &mut index), 65535);
    assert_eq!(read_varint(&buf, &mut index), 2305843009213693953);

    assert_eq!(index, 16);

    let mut buf2 = [0; 16];
    let mut index2 = 0;

    write_varint(&mut buf2, &mut index2, 63);
    write_varint(&mut buf2, &mut index2, 12);
    write_varint(&mut buf2, &mut index2, 511);
    write_varint(&mut buf2, &mut index2, 65535);
    write_varint(&mut buf2, &mut index2, 2305843009213693953);

    assert_eq!(index2, 16);
    assert_eq!(buf, buf2);
}

#[tokio::test]
#[ignore = "network"]
async fn udp_recv() {
    let mut buff = BytesMut::new();
    buff.resize(1800, 0);

    let sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
    sock.set_nonblocking(true).unwrap();
    
    #[cfg(unix)]
    sock.reuse_port().unwrap();
    sock.reuse_address().unwrap(); 
    
    sock.bind(&SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0,0,0,0), 2000)).into()).unwrap();

    let tsock = UdpSocket::from_std(sock.into()).unwrap();

    let (len, addr) = tsock.recv_from(&mut buff).await.unwrap();
    println!("{addr:?} -> {len}");
    let string = String::from_utf8_lossy(&buff[..len]);
    println!("{string:?}");
}

#[tokio::test]
#[ignore = "network"]
async fn quic_listen() {
    let addr = socket2::SockAddr::from(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0,0,0,0), 2000)));

    let quic = QuicListener::bind(&addr).unwrap();
    quic.listen(4096).await.unwrap();
    
}

