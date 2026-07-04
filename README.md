# photon
my own ffi http library 

## Bindings
The FFI bindings are in a seperate repo [samicpp/httplib-bindings](https://github.com/samicpp/httplib-bindings)

## Crate features
| feature      | requires | desciption                       |
|--------------|----------|----------------------------------|
| asyncffi     | -        | enables asynchronous ffi exports |
| unix-sockets | asyncffi | allows unix sockets in ffi types |
| ring         | asyncffi | uses ring provider               |
| aws-lc-rs    | asyncffi | uses aws provider                |

## TODO

### Protocols
#### Server-Side
- [x] HTTP/1.1
- [x] WebSocket
- [x] HPACK
- [x] HTTP/2
- [ ] QPACK 
- [ ] HTTP/3 (first using quinn, later my own)
- [ ] QUIC (replaces quinn)

#### Client-Side
- [x] HTTP/1.1
- [x] WebSocket
- [x] HPACK
- [x] HTTP/2
- [ ] QPACK 
- [ ] HTTP/3 (first using quinn, later my own)
- [ ] QUIC (replaces quinn)

### Features
- [x] server support TLS
- [x] client support TLS
- [x] FFI compatible
- [x] custom error enums
- [x] allow Http2Frame to live on stack
- [x] change HttpSocket.set_status status type to ~~`Cow<'static, str>`~~ `String`
- [x] change HttpSocket.set_header value type to ~~`Cow<'static, str>`~~ `String`
- [x] add type in `FfiFuture<T>` in asyncffi ffi definitions
- [x] implement `Serialize` and `Deserialize` for HttpClient and HttpResponse
- [x] add example code
- [ ] allow configuring tokio runtime
- [ ] ~~rewrite http to use `futures` instead of `tokio`~~
- [ ] ~~allow compiling with different async runtimes~~
- [ ] add builtin content compressions (gzip, deflate, brotli, zstd)
- [ ] add header size limit for reading connections to prevent attacks
- [ ] allow setting protocol correctness enforcement
- [ ] support no-std
- [ ] change how headers are stored internally for near zero allocation header storage

## Examples

examples can be found in [/examples](/examples) <br/>

[HTTP/1.1 server](/examples/tcp_http1_server.rs)
```rust
use photon::http1::server::Http1Socket;
use tokio::net::TcpListener;


#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();

    loop {
        let (stream, address) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            dbg!(address);
            let mut http1 = Http1Socket::new(stream, 8 * 1024);
            http1.read_until_complete().await.unwrap();

            http1.set_header("Server", "example".to_string());
            http1.set_header("Content-Type", "text/html".to_string());

            http1.close(br#"<!DOCTYPE html>
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
</html>"#).await.unwrap();
        });
    }
}
```

[HTTP/2 server](/examples/tcp_http2_server.rs)
```rust
use photon::{http2::{core::Http2Settings, session::Http2Session}, shared::LibError};
use tokio::net::TcpListener;


#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();

    loop {
        let (stream, address) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            dbg!(address);
            let http2 = Http2Session::new(stream);
            http2.read_preface().await.unwrap();
            http2.send_settings(Http2Settings::DEFAULT_NO_PUSH).await.unwrap(); // DEFAULT, DEFAULT_NO_PUSH, MAXIMUM
            http2.send_ping(false, b"hearbeat").await.unwrap();

            loop {
                let frame = http2.read_frame().await.unwrap();
                println!("\x1b[36m{:?}\x1b[0m ({}) {:?}", frame.ftype, frame.source.len(), &frame.source[..29.min(frame.source.len())]);
                
                match http2.handle(frame).await {
                    Ok(Some(id)) => {
                        http2.send_headers(id, false, &[
                            (b":status", b"200"),
                            (b"server", b"example"),
                            (b"content-type", b"text/html"),
                            (b"content-length", b"300"),
                        ]).await.unwrap();
                        http2.send_data(id, true, br#"<!DOCTYPE html>
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
</html>"#).await.unwrap();
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
        });
    }
}
```

[HTTP/1.1 with H2C and HTTP/2 Prior Knowledge](/examples/tcp_http_h2c_server.rs)
```rust
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
```
