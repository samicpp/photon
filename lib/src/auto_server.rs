// use std::{os::unix::net::SocketAddr, sync::Arc};

// use rustls::ServerConfig;
// use tokio::{net::TcpListener, sync::{mpsc, watch}};
// use tokio_rustls::TlsAcceptor;



// pub struct Server {
//     pub listener: TcpListener,
//     pub tls: Option<Arc<TlsAcceptor>>,

//     pub tcp_detect: bool,
//     pub blocked: Vec<SocketAddr>,
// }
// impl Server {
//     pub fn new(listener: TcpListener, conf: Option<Arc<ServerConfig>>) -> Self {
//         Self {
//             tls: conf.map(|c| Arc::new(TlsAcceptor::from(c))),
//             listener,
//             tcp_detect: true,
//             blocked: Vec::new(),
//         }
//     }

//     pub fn accept(&self, mut cancel: watch::Receiver<bool>, socktx: mpsc::Sender<Result<>>) -> Result<> {

//     }
// }