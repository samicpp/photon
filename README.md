# httplib
my own ffi http library 

## Bindings
I will make FFI bindings to a bunch of languages/frameworks for this library


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
- [ ] rewrite http to use `futures` instead of `tokio`
- [ ] allow compiling with different async runtimes


## Examples

HTTP/1.1 server
```rust
use http::http1::server::Http1Socket;
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

            http1.set_header("Server", "example");
            http1.set_header("Content-Type", "text/html");

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