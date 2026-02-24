#![allow(dead_code)]
#![allow(unused_imports)]
#![cfg(test)]

use std::{net::{SocketAddr, ToSocketAddrs}, sync::{Arc, atomic::AtomicBool}, time::Duration};
use http::{extra::PolyHttpSocket, http1::{client::Http1Request, server::Http1Socket}, http2::{core::Http2Settings, session::Http2Session}, shared::{HttpRequest, HttpSocket, HttpVersion, LibError, ReadStream, WriteStream}, websocket::{core::WebSocketOpcode, socket::WebSocket}};
use tokio::{io::AsyncReadExt, net::TcpListener, sync::Mutex};
use crate::{clients::{tcp_connect, tls_upgrade}, /*httpcpp::server_test,*/ servers::tcp_serve};


#[ignore = "requires user input"]
#[tokio::test]
async fn serve_tcp(){
    // tcp_serve("0.0.0.0:1024".to_owned(), |a,h| handler(a,h)).await.unwrap();
    let tcp = TcpListener::bind("0.0.0.0:1024".to_owned()).await.unwrap();
    let (sock, addr) = tcp.accept().await.unwrap();
    let mut http = Http1Socket::new(sock, 8 * 1024);
    // let mut http = PolyHttpSocket::Http1(http);
    
    println!("{}", addr);
    let mut client = http.read_until_complete().await.unwrap().clone();
    let body = client.body;
    client.body = Vec::with_capacity(0);
    dbg!(&client);
    println!("body = {}", String::from_utf8_lossy(&body));

    if client.valid{
        assert_eq!(client.head_complete, true);
        assert_eq!(client.body_complete, true);
    }
    
    if !client.valid {
        http.set_status(400, "Bad Request".to_owned());
        http.close(b"fix your client").await.unwrap();
    }
    else if client.version.is_unknown() {
        http.version_override = Some(HttpVersion::Http11);
        http.set_status(400, "Bad Request".to_owned());
        http.close(format!("\"{}\" is not a valid version", if let HttpVersion::Unknown(Some(u)) = client.version { u.clone() } else { "???".to_owned() }).as_bytes()).await.unwrap();
    }
    else if client.method.is_unknown() {
        http.set_status(405, "Method Not Allowed".to_owned());
        http.set_header("Allow", "GET, HEAD, POST, PUT, DELETE, CONNECT, OPTIONS, TRACE");
        http.close(b"erm, what are you trying to do?").await.unwrap();
    }
    else if client.version.is_http11() && client.host.is_none(){
        http.set_status(400, "Bad Request".to_owned());
        http.close(b"what're you connecting to, what host?").await.unwrap();
    }
    else {
        http.set_status(200, "OK".to_owned());
        http.set_header("Content-Type", "text/plain");
        http.close(b"everything's alright").await.unwrap();
    }
}


// #[ignore = "user interaction"]
// #[test]
// fn test_over_ffi(){
//     std::thread::spawn(move || {
//         unsafe {
//             assert_eq!(server_test(), 0);
//         }
//     }).join().unwrap();
// }


#[ignore = "uses network"]
#[tokio::test]
async fn request_google(){
    let tcp = tcp_connect("google.com:443").await.unwrap();
    let tls = tls_upgrade(tcp, "www.google.com".to_owned(), vec![b"http/1.1".to_vec()]).await.unwrap();
    let mut req = Http1Request::new(Box::new(tls), 8 * 1024);

    req.set_path("/".to_owned());
    req.set_header("Host", "www.google.com");
    req.send(b"").await.unwrap();
    let _ = req.read_until_complete().await.unwrap();
    let body = req.response.body;
    req.response.body = Vec::new();
    
    println!("body.len() == {}", body.len());
    dbg!(&req);
    dbg!(&req.response);
    println!("{}", String::from_utf8_lossy(&body));
}


#[ignore = "requires user interaction"]
#[tokio::test]
async fn ws_upgrade(){
    let listener = TcpListener::bind("0.0.0.0:4096").await.unwrap();
    
    loop {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut http = Http1Socket::new(tcp, 8 * 1024);
        
        let client = http.read_until_head_complete().await.unwrap();

        let connection = client.headers.get("connection").and_then(|v| Some(&v[0]));
        let upgrade = client.headers.get("upgrade").and_then(|v| Some(&v[0]));
        let secwskey = client.headers.get("sec-websocket-key").and_then(|v| Some(&v[0]));

        // dbg!(client);

        if 
            let Some(connection) = connection && connection == "Upgrade" && 
            let Some(upgrade) = upgrade && upgrade == "websocket" &&
            let Some(secwskey) = secwskey
        {
            let wskey = secwskey.as_bytes().to_vec();
            let ws = http.websocket_with_key(wskey).await.unwrap();
            // let open = true;
            println!("ws upgrade");

            loop {
                match ws.read_frame().await {
                    Ok(mut frame) => {
                        frame.unmask_in_place();
                        println!("frame \x1b[36m{:?}\x1b[0m", frame.opcode);

                        match frame.opcode {
                            WebSocketOpcode::Continuation |
                            WebSocketOpcode::Text | 
                            WebSocketOpcode::Binary => ws.send_text(frame.get_payload()).await.unwrap(),

                            WebSocketOpcode::Ping => ws.send_pong(frame.get_payload()).await.unwrap(),

                            WebSocketOpcode::ConnectionClose => {
                                let code = u16::from_be_bytes([frame.get_payload()[0], frame.get_payload()[1]]);
                                println!("client closes with {}", code);

                                ws.send_close(4455, b"client closed").await.unwrap();
                                break;
                            },

                            _ => (),
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        break;
                    },
                }
            }

            break;
        }
        else {
            http.set_status(418, "I'm a teapot".to_owned());
            http.close(b"upgrade required").await.unwrap();
        }
    }
}


#[ignore = "requires user interaction"]
#[tokio::test]
async fn ws_mirror(){
    let listener = TcpListener::bind("0.0.0.0:4097").await.unwrap();
    let other: Mutex<Option<Arc<WebSocket<tokio::io::BufReader<tokio::io::ReadHalf<tokio::net::TcpStream>>, tokio::io::WriteHalf<tokio::net::TcpStream>>>>> = Mutex::new(None);
    let other = Arc::new(other);
    // let done = AtomicBool::new(false);

    loop {
        let (tcp, _) = listener.accept().await.unwrap();
        let other = other.clone();

        tokio::spawn(async move {

            let mut http = Http1Socket::new(tcp, 8 * 1024);
            
            let client = http.read_until_head_complete().await.unwrap();

            if client.headers.contains_key("sec-websocket-key") {
                let wso = Arc::new(http.websocket().await.unwrap());
                println!("ws upgrade");

                let mut guard = other.lock().await;

                if let Some(othero) = &*guard {
                    let ws = wso.clone();
                    let other = othero.clone();
                    let t0 = tokio::spawn(async move {
                        loop {
                            match other.read_frame().await {
                                Ok(mut frame) => {
                                    frame.unmask_in_place();
                                    println!("other: frame \x1b[36m{:?}\x1b[0m", frame.opcode);

                                    match frame.opcode {
                                        WebSocketOpcode::Text => ws.send_text(frame.get_payload()).await.unwrap(),
                                        WebSocketOpcode::Binary => ws.send_binary(frame.get_payload()).await.unwrap(),

                                        WebSocketOpcode::Ping => ws.send_ping(frame.get_payload()).await.unwrap(),
                                        WebSocketOpcode::Pong => ws.send_pong(frame.get_payload()).await.unwrap(),

                                        WebSocketOpcode::ConnectionClose => {
                                            let code = u16::from_be_bytes([frame.get_payload()[0], frame.get_payload()[1]]);
                                            let msg = &frame.get_payload()[2..];
                                            println!("other closes with {}: {}", code, String::from_utf8_lossy(msg));

                                            ws.send_close(code, msg).await.unwrap();
                                            break;
                                        },

                                        _ => (),
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{e}");
                                    other.send_close(1006, b"weird thing happened with other").await.unwrap();
                                    break;
                                },
                            }
                        }
                    });

                    let ws = wso.clone();
                    let other = othero.clone();
                    let t1 = tokio::spawn(async move {
                        loop {
                            match ws.read_frame().await {
                                Ok(mut frame) => {
                                    frame.unmask_in_place();
                                    println!("ws: frame \x1b[36m{:?}\x1b[0m", frame.opcode);

                                    match frame.opcode {
                                        WebSocketOpcode::Text => other.send_text(frame.get_payload()).await.unwrap(),
                                        WebSocketOpcode::Binary => other.send_binary(frame.get_payload()).await.unwrap(),

                                        WebSocketOpcode::Ping => other.send_ping(frame.get_payload()).await.unwrap(),
                                        WebSocketOpcode::Pong => other.send_pong(frame.get_payload()).await.unwrap(),

                                        WebSocketOpcode::ConnectionClose => {
                                            let code = u16::from_be_bytes([frame.get_payload()[0], frame.get_payload()[1]]);
                                            let msg = &frame.get_payload()[2..];
                                            println!("other closes with {}: {}", code, String::from_utf8_lossy(msg));

                                            other.send_close(code, msg).await.unwrap();
                                            break;
                                        },

                                        _ => (),
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{e}");
                                    other.send_close(1006, b"weird thing happened with ws").await.unwrap();
                                    break;
                                },
                            }
                        }
                    });


                    let r0 = t0.await;
                    let r1 = t1.await;
                    let _ = r0.unwrap();
                    let _ = r1.unwrap();
                } 
                else {
                    *guard = Some(wso);
                }
            }
            else {
                http.set_status(418, "I'm a teapot".to_owned());
                http.close(b"upgrade required").await.unwrap();
            }
        });

        println!("loop");
    }
}


#[ignore = "requires user interaction"]
#[tokio::test]
async fn http2_test(){
    let listener = TcpListener::bind("0.0.0.0:8196").await.unwrap();
    
    let (sock, addr) = listener.accept().await.unwrap();
    println!("\x1b[38;5;8m{addr:?}\x1b[0m");

    let h2 = Http2Session::new_server(sock);
    let h2 = Arc::new(h2);
    println!("created session");

    assert_eq!(h2.read_preface().await.unwrap(), true);
    h2.send_settings(Http2Settings::default_no_push()).await.unwrap();

    let opened;
    loop {
        let frame = h2.read_frame().await.unwrap();
        println!("\x1b[36m{:?}\x1b[0m {:?}", frame.ftype, frame.source);
        let id = h2.handle(frame).await.unwrap();

        if let Some(id) = id {
            opened = id;
            break;
        }
    }
    println!("stream was opened {opened}");


    let cl = h2.clone();
    let join = tokio::spawn(async move {
        let h2 = cl;
        
        loop {
            match h2.next().await {
                Err(LibError::Io(_)) => break,
                res => res.unwrap(),
            };
        }
        println!("background done");
    });
    println!("background loop");

    h2.send_headers(opened, false, &[(b":status", b"200"), (b"content-type", b"text/plain")]).await.unwrap();
    h2.send_data(opened, true, b"herro world").await.unwrap();
    // h2.send_goaway(0, 0, b"shutdown").await.unwrap();
    println!("done");

    // tokio::time::sleep(Duration::from_millis(1000)).await;

    join.await.unwrap();
}