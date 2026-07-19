#![cfg(test)]

use std::sync::atomic::Ordering;

use crate::{http1::{client::Http1Request, server::Http1Socket}, http2::{core::{Http2Frame, Http2FrameType, Http2Settings}, hpack::{Biterator, HeaderType, decoder::Decoder, encoder::Encoder}, session::Http2Session}, shared::ByteSource, websocket::core::WebSocketFrame};

#[test]
fn two_is_two(){
    assert!(2 == 2)
}

#[tokio::test]
async fn server_client(){
    let (client, server) = tokio::io::duplex(64 * 1024);

    let mut client = Http1Request::new(client, 8 * 1024);
    let mut server = Http1Socket::new(server, 8 * 1024);

    client.path = "/test".to_owned();
    client.send(b"").await.unwrap();
    server.read_until_complete().await.unwrap();
    
    assert_eq!(client.path, server.client.path);
    assert_eq!(client.method, server.client.method);
    assert_eq!(client.version, server.client.version);

    server.close(b"test").await.unwrap();
    client.read_until_complete().await.unwrap();

    assert_eq!(server.code, client.response.code);
    assert!("test".as_bytes() == &client.response.body);
    assert_eq!(server.status, client.response.status.trim());
}

#[test]
fn num_sizes(){
    let int8: u8 = 0;
    let int16: u16 = 0;
    let int32: u32 = 0;
    let int64: u64 = 0;
    let intptr: usize = 0;

    assert_eq!(int8.to_be_bytes().len(), 1);
    assert_eq!(int16.to_be_bytes().len(), 2);
    assert_eq!(int32.to_be_bytes().len(), 4);
    assert_eq!(int64.to_be_bytes().len(), 8);
    assert_eq!(intptr.to_be_bytes().len(), usize::BITS as usize / 8);
}

#[test]
fn websocket_frame(){
    let frame_buff = vec![
        0x82, 0xff, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0d,
        0x12, 0x34, 0x56, 0x78,
        0x5a, 0x51, 0x3a, 0x14, 0x7d, 0x18, 0x76, 0x2f, 0x7d, 0x46, 0x3a, 0x1c, 0x6c,
    ];

    let mut frame = WebSocketFrame::from_owned(frame_buff).unwrap();
    let s = String::from_utf8_lossy(frame.unmask_in_place()).to_string();
    let frame = frame;

    assert_eq!(frame.fin, true);
    assert_eq!(frame.rsv, 0);
    assert_eq!(frame.opcode_byte, 2);
    assert_eq!(frame.masked, true);
    assert_eq!(frame.len, 127);
    assert_eq!(frame.ext_len, 13);
    assert_eq!(s, "Hello, World~");

    // dbg!(frame);
}

#[tokio::test]
async fn websocket(){
    let (client, server) = tokio::io::duplex(64 * 1024);

    let mut client = Http1Request::new(client, 8 * 1024);
    let mut server = Http1Socket::new(server, 8 * 1024);

    let f0 = tokio::spawn(async move {
        client.set_header("Host", "localhost".into());
        client.path = "/test".to_owned();
        let cws = client.websocket_strict().await.unwrap();

        let mut mask = [0u8; 4];
        rand::fill(&mut mask);
        cws.send_text_masked(&mask, b"bin").await.unwrap();

        let frame = cws.read_frame().await.unwrap();
        assert_eq!(frame.fin, true);
        assert_eq!(frame.rsv, 0);
        assert_eq!(frame.opcode_byte, 8);
        assert_eq!(frame.masked, false);
        assert_eq!(frame.len, 9);
        assert_eq!(frame.ext_len, 0);
        assert_eq!(frame.get_payload(), b"\x03\xe8message");
        cws.send_close(1000, b"message").await.unwrap();
    });


    let _ = server.read_until_complete().await.unwrap();
    assert_eq!(server.client.headers.contains_key("sec-websocket-key"), true);
    
    let sws = server.websocket().await.unwrap();
    let mut frame = sws.read_frame().await.unwrap();
    frame.unmask_in_place();
    assert_eq!(frame.fin, true);
    assert_eq!(frame.rsv, 0);
    assert_eq!(frame.opcode_byte, 1);
    assert_eq!(frame.masked, true);
    assert_eq!(frame.len, 3);
    assert_eq!(frame.ext_len, 0);
    assert_eq!(frame.get_payload(), b"bin");
    sws.send_close(1000, b"message").await.unwrap();

    f0.await.unwrap();
}

#[test]
fn biterator(){
    let bytes = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let bits = [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true, false, false, false, false, false, false, true, false, false, false, false, false, false, false, true, true, false, false, false, false, false, true, false, false, false, false, false, false, false, true, false, true, false, false, false, false, false, true, true, false, false, false, false, false, false, true, true, true, false, false, false, false, true, false, false, false, false, false, false, false, true, false, false, true];
    let biter = Biterator::new(bytes.iter()).collect::<Vec<bool>>();
    
    assert_eq!(&bits as &[bool], biter.as_slice());
}

#[test]
fn hpack_decode(){
    let mut decoder: Decoder<'static> = Decoder::new(4096);
    // let encoded = [0x82, 0x85, 0x40, 0x85, 0x35, 0x52, 0x17, 0xC9, 0x64, 0x85, 0x9C, 0xA3, 0x90, 0xB6, 0x7F, 0x40, 0x88, 0xA8, 0xE9, 0x50, 0xD5, 0x48, 0x5F, 0x25, 0x93, 0x85, 0x9C, 0xA3, 0x90, 0xB6, 0x7F];
    let encoded = [
        0x82,
        0x85,
        0x26,
        0x40, 0x85, 0x35, 0x52, 0x17, 0xc9, 0x64, 0x85, 0x9c, 0xa3, 0x90, 0xb6, 0x7f,
        0x0, 0x82, 0xa8, 0xe9, 0x85, 0x35, 0x52, 0x17, 0xc9, 0x64,
        0x10, 0x84, 0xa8, 0xbd, 0xcb, 0x67, 0x85, 0x35, 0x52, 0x17, 0xc9, 0x64,
        0x0, 0x86, 0x9e, 0xd9, 0x65, 0xa4, 0x75, 0x7f, 0x85, 0x2d, 0x44, 0x3c, 0x85, 0x93,
        0x0, 0x88, 0xa8, 0xe9, 0x52, 0x7b, 0x65, 0x96, 0x91, 0xd5, 0x85, 0x2d, 0x44, 0x3c, 0x85, 0x93,
    ];
    let decoded: &[(&[u8], &[u8])] = &[(b":method", b"GET"), (b":path", b"/index.html"), (b"indexed", b"header"), (b"not", b"indexed"), (b"never", b"indexed"), (b"huffman", b"encoded"), (b"not huffman", b"encoded")];

    /* 
    // js code used to format
    const enc = new Uint8Array([0x82, 0x85, 0x40, 0x85, 0x35, 0x52, 0x17, 0xC9, 0x64, 0x85, 0x9C, 0xA3, 0x90, 0xB6, 0x7F, 0x0, 0x82, 0xA8, 0xE9, 0x85, 0x35, 0x52, 0x17, 0xC9, 0x64, 0x10, 0x84, 0xA8, 0xBD, 0xCB, 0x67, 0x85, 0x35, 0x52, 0x17, 0xC9, 0x64, 0x0, 0x86, 0x9E, 0xD9, 0x65, 0xA4, 0x75, 0x7F, 0x85, 0x2D, 0x44, 0x3C, 0x85, 0x93, 0x0, 0x88, 0xA8, 0xE9, 0x52, 0x7B, 0x65, 0x96, 0x91, 0xD5, 0x85, 0x2D, 0x44, 0x3C, 0x85, 0x93,])
    const sizes = [1,1,13,10,13,12,14,16]
    const posses = [0,1,2,15,25,37,51]
    const ranges = [[],[],[],[],[],[],[],]
    let range = 0

    enc.forEach((e,i)=>{
        let p = posses.indexOf(i);
        if (p > -1) range = p;
        ranges[range].push("0x"+e.toString(16))
    })

    copy(ranges.map(e=>e.join(", ")).join(",\n"))
    */

    // let dec = decoder.decode(&encoded).unwrap();;
    // let dec_ref = dec.iter().map(|(h,v)| (h.as_slice(), v.as_slice())).collect::<Vec<(&[u8], &[u8])>>();
    
    let mut dec = Vec::new();
    let mut pos = 0;

    while pos < encoded.len() {
        let opos = pos;
        let last = decoder.decode(&encoded, &mut pos).unwrap();
        let htyp = last.0;
        let last = if last.0 != HeaderType::TableSizeChange { (last.1, last.2) } else { continue; };

        dec.push(last.clone());
 
        // let read = &encoded[opos..pos];
        println!("[{} - {} : {}] = \x1b[38;5;8m{:?}\x1b[0m({}, {})", opos, pos, pos - opos, htyp, String::from_utf8_lossy(&last.0), String::from_utf8_lossy(&last.1))
    }

    let dec_ref = dec.iter().map(|(h,v)| (h.as_slice(), v.as_slice())).collect::<Vec<(&[u8], &[u8])>>();

    assert_eq!(decoded, dec_ref.as_slice());
}

#[test]
fn hpack_encode(){
    let mut encoder: Encoder<'static> = Encoder::new(4096);
    let encoded = [
        0x82,
        0x85,
        0x40, 0x85, 0x35, 0x52, 0x17, 0xc9, 0x64, 0x85, 0x9c, 0xa3, 0x90, 0xb6, 0x7f,
        0x0, 0x82, 0xa8, 0xe9, 0x85, 0x35, 0x52, 0x17, 0xc9, 0x64,
        0x10, 0x84, 0xa8, 0xbd, 0xcb, 0x67, 0x85, 0x35, 0x52, 0x17, 0xc9, 0x64,
        0x0, 0x86, 0x9e, 0xd9, 0x65, 0xa4, 0x75, 0x7f, 0x85, 0x2d, 0x44, 0x3c, 0x85, 0x93,
        0x0, 0xb, 0x6e, 0x6f, 0x74, 0x20, 0x68, 0x75, 0x66, 0x66, 0x6d, 0x61, 0x6e, 0x7, 0x65, 0x6e, 0x63, 0x6f, 0x64, 0x65, 0x64
    ];
    let decoded: &[(HeaderType, &[u8], &[u8], Option<bool>)] = &[
        (HeaderType::Lookup, b":method", b"GET", None), 
        (HeaderType::Lookup, b":path", b"/index.html", None), 
        (HeaderType::Indexed, b"indexed", b"header", None), 
        (HeaderType::NotIndexed, b"not", b"indexed", None), 
        (HeaderType::NeverIndexed, b"never", b"indexed", None), 
        (HeaderType::NotIndexed, b"huffman", b"encoded", Some(true)), 
        (HeaderType::NotIndexed, b"not huffman", b"encoded", Some(false)),
    ];

    let mut buff = Vec::new();

    for &h in decoded {
        println!("encoding header");
        encoder.encode(&mut buff, h.0, h.1, h.2, h.3).unwrap();
    }

    dbg!(&buff);
    assert_eq!(buff.as_slice(), encoded);

}

#[test]
fn http2_frame() {
    let frame_raw = [
        0u8, 0, 19, 
        0, 1 | 8 | 32, 
        0, 0, 0, 3, 
        2,
        0, 0, 0, 1, 2,
        0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        0x68, 0x69,
    ];
    
    let frame = Http2Frame::from(ByteSource::Stack(frame_raw)).unwrap();
    let frame_buff = Http2Frame::create_heap(frame.ftype, frame.flags, frame.stream_id, Some(frame.get_priority()), Some(frame.get_payload()), Some(frame.get_padding())).unwrap();

    assert_eq!(frame.is_end_headers(), false);
    assert_eq!(frame.is_end_stream(), true);
    assert_eq!(frame.is_padded(), true);
    assert_eq!(frame.is_priority(), true);
    assert_eq!(frame.length, 19);
    assert_eq!(frame.ftype, Http2FrameType::Data);
    assert_eq!(frame.type_byte, 0);
    assert_eq!(frame.flags, 41);
    assert_eq!(frame.stream_id, 3);
    assert_eq!(frame.get_priority(), &[0, 0, 0, 1, 2]);
    assert_eq!(frame.get_payload(), b"hello world");
    assert_eq!(frame.get_padding(), b"hi");

    assert_eq!(frame_buff.as_slice(), &frame_raw);

}

#[tokio::test]
async fn http2() {
    let (client, server) = tokio::io::duplex(64 * 1024);

    let client = Http2Session::new_client(client);
    let server = Http2Session::new_server(server);


    client.send_preface().await.unwrap();
    assert_eq!(server.read_preface().await.unwrap(), true);

    client.send_settings(Http2Settings::default()).await.unwrap();
    server.send_settings(Http2Settings::default()).await.unwrap();
    
    let frame = client.read_frame().await.unwrap();
    assert_eq!(frame.ftype, Http2FrameType::Settings);
    client.handle(frame).await.unwrap();

    let frame = server.read_frame().await.unwrap();
    assert_eq!(frame.ftype, Http2FrameType::Settings);
    server.handle(frame).await.unwrap();

    client.next().await.unwrap();
    server.next().await.unwrap();

    let stream_id = client.open_stream().unwrap();
    assert_eq!(stream_id, 1);
    client.send_headers(stream_id, false, &[
        (b":method", b"POST"),
        (b":scheme", b"https"),
        (b":authority", b"localhost"),
        (b":path", b"/index.html"),
        (b"accept", b"*/*"),
        (b"content-type", b"text/plain"),
        (b"content-length", b"12"),
    ]).await.unwrap();
    client.send_data(stream_id, true, b"hello, world").await.unwrap();

    let opened = server.next().await.unwrap().unwrap(); 
    assert_eq!(server.next().await.unwrap(), None);
    assert_eq!(opened, 1);

    client.next().await.unwrap(); // window update
    client.next().await.unwrap(); // window update
    
    server.send_headers(opened, false, &[
        (b":status", b"200"),
        (b"content-type", b"text/plain"),
        (b"content-length", b"12"),
    ]).await.unwrap();
    server.send_data(opened, true, b"hello world.").await.unwrap();
    
    client.next().await.unwrap();
    client.next().await.unwrap();
    server.next().await.unwrap(); // window update
    server.next().await.unwrap(); // window update
    
    let promise = server.open_stream().unwrap();
    assert_eq!(promise, 2);
    server.send_push_promise(opened, promise, &[
        (b":method", b"POST"),
        (b":scheme", b"https"),
        (b":authority", b"localhost"),
        (b":path", b"/login.php"),
    ]).await.unwrap();
    server.send_headers(promise, false, &[
        (b":status", b"200"),
        (b"content-type", b"text/json"),
        (b"content-length", b"17"),
    ]).await.unwrap();
    server.send_data(promise, true, b"\"some rust thing\"").await.unwrap();

    let promised = client.next().await.unwrap().unwrap();
    assert_eq!(promised, 2);
    client.next().await.unwrap();
    client.next().await.unwrap();

    server.next().await.unwrap(); // window update
    server.next().await.unwrap(); // window update


    client.send_goaway(0, 0, b"shutdown").await.unwrap();
    server.next().await.unwrap();
    assert_eq!(server.goaway.load(Ordering::SeqCst), true);

    drop(server);
    drop(client);
}