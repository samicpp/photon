use std::sync::Arc;

use photon::{http1::server::Http1Socket, http2::{core::Http2Settings, server::Http2Socket}, shared::{HttpMethod, HttpSocket, HttpVersion, LibError, LibResult}};
use tokio::{io::AsyncReadExt, net::TcpListener};


#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();

    loop {
        let (stream, address) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            dbg!(address);
            let mut http1 = Http1Socket::new(stream, 8 * 1024);
            let client = http1.read_until_complete().await.unwrap();


            if 
                client.method == HttpMethod::Unknown(Some("PRI".to_owned())) && 
                client.path == "*" && 
                client.version == HttpVersion::Unknown(Some("HTTP/2.0".to_owned())) && 
                client.headers.len() == 0 
            {
                http1.netr.read_exact(&mut [0; 6]).await.unwrap();
                let http2 = Arc::new(http1.http2_direct(Http2Settings::DEFAULT_NO_PUSH));

                http2.send_settings(Http2Settings::MAXIMUM).await.unwrap();
                http2.send_ping(false, b"hearbeat").await.unwrap();

                loop {
                    let frame = http2.read_frame().await.unwrap();
                    println!("\x1b[36m{:?}\x1b[0m ({}) {:?}", frame.ftype, frame.source.len(), &frame.source[..29.min(frame.source.len())]);
                    
                    match http2.handle(frame).await {
                        Ok(Some(id)) => {
                            let http = Http2Socket::new(id, http2.clone()).unwrap();
                            handler(http).await.unwrap()
                        },
                        Ok(None) => (),
                        Err(err @ (LibError::InvalidFrame | LibError::InvalidStream | LibError::ProtocolError | LibError::Huffman(_))) => {
                            eprintln!("{err}");
                            http2.send_goaway(0, 1, b"protocol error").await.unwrap();
                            break;
                        },
                        Err(err) => {
                            eprintln!("{err}");
                            break;
                        },
                    }
                    if http2.goaway.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                }
            } 
            else if 
                let Some(up) = client.headers.get("upgrade") && 
                up[0].to_lowercase() == "h2c" 
            {
                let h2c = Arc::new(http1.h2c(Some(Http2Settings::MAXIMUM)).await.unwrap());
                h2c.read_preface().await.unwrap();
                h2c.send_settings(Http2Settings::MAXIMUM).await.unwrap();
                h2c.send_ping(false, b"hearbeat").await.unwrap();

                let http = Http2Socket::new(1, h2c.clone()).unwrap();

                tokio::spawn(async move {
                    handler(http).await.unwrap()
                });

                loop {
                    let frame = h2c.read_frame().await.unwrap();
                    println!("\x1b[36m{:?}\x1b[0m ({}) {:?}", frame.ftype, frame.source.len(), &frame.source[..29.min(frame.source.len())]);
                    
                    match h2c.handle(frame).await {
                        Ok(Some(id)) => {
                            let http = Http2Socket::new(id, h2c.clone()).unwrap();
                            handler(http).await.unwrap()
                        },
                        Ok(None) => (),
                        Err(err @ (LibError::InvalidFrame | LibError::InvalidStream | LibError::ProtocolError | LibError::Huffman(_))) => {
                            eprintln!("{err}");
                            h2c.send_goaway(0, 1, b"protocol error").await.unwrap();
                            break;
                        },
                        Err(err) => {
                            eprintln!("{err}");
                            break;
                        },
                    }
                    if h2c.goaway.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                }
            }
            else {
                handler(http1).await.unwrap()
            }
        });
    }
}

async fn handler<H: HttpSocket>(mut http: H) -> LibResult<()> {
    http.set_header("Server", "example".to_string());
    http.set_header("Content-Type", "text/html".to_string());

    http.close(br#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document</title>
    <style> body { font-family: Arial, Helvetica, sans-serif; } </style>
</head>
<body>
    <h1>Hello, World!</h1>
</body>
</html>"#).await
}