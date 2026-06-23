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