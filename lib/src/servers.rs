use std::{io, net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use http::{extra::PolyHttpSocket, http1::server::Http1Socket, http2::PREFACE};
use rustls::{ServerConfig, server::{ClientHello, ResolvesServerCert}, sign::CertifiedKey};
use tokio::{io::{ReadHalf, WriteHalf}, net::{TcpListener, TcpStream}};
use tokio_rustls::{TlsAcceptor, server::TlsStream};

use crate::DynStream;



pub async fn tcp_serve<F, Fut, O>(address: String, handler: F) -> std::io::Result<()>
where 
    F: Fn(SocketAddr, PolyHttpSocket<ReadHalf<TcpStream>, WriteHalf<TcpStream>>) -> Fut + Send + Clone + Copy + Sync + 'static,
    Fut: Future<Output = O> + Send + 'static,
{
    let listener = TcpListener::bind(address).await?;

    loop{
        let (socket, adddress) = listener.accept().await?;
        tokio::spawn(async move{
            let http = Http1Socket::new(socket, 8 * 1024);
            // let http: Box<dyn HttpSocket + Send> = Box::new(http);
            let http = PolyHttpSocket::Http1(http);
            handler(adddress, http).await;
        });
    }
}

pub type DynHttpSocket = PolyHttpSocket<ReadHalf<DynStream>, WriteHalf<DynStream>>;

// pub struct TcpServer{
//     // cb: Arc<dyn Fn(SocketAddr, PolyHttpSocket<ReadHalf<TcpStream>, WriteHalf<TcpStream>>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync + 'static>,
//     listener: TcpListener,
// }
// impl TcpServer{
//     pub async fn new(address: String) -> std::io::Result<Self>{
//         Ok(Self {
//             listener: TcpListener::bind(address).await?,
//         })
//     }
//     pub async fn accept(&mut self) -> io::Result<(SocketAddr, TcpStream)> {
//             let (sock, addr) = self.listener.accept().await?;
//             // let http = Http1Socket::new(sock, 8 * 1024);
//             // let http = PolyHttpSocket::Http1(http);
//             Ok((addr, sock))
//     }
// }

pub async fn detect_prot(tcp: &mut TcpStream) -> u8{
    let mut peek = [0u8; 24];
    let _ = tcp.peek(&mut peek).await;

    if peek == PREFACE{
        1 // http2
    }
    else if peek[0] == 22{
        2 // tls
    }
    else {
        0 // idk
    }
}

#[derive(Debug, Clone)]
pub struct TlsCertSelector{
    pub default: Option<Arc<CertifiedKey>>,
    pub sni_match: DashMap<String, Arc<CertifiedKey>>
}
impl TlsCertSelector{
    pub fn new() -> Self{
        Self {
            default: None,
            sni_match: DashMap::new(),
        }
    }
    pub fn with_default(default: CertifiedKey) -> Self{
        Self {
            default: Some(Arc::new(default)),
            sni_match: DashMap::new(),
        }
    }
    pub fn to_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
    pub fn to_server_conf(self) -> ServerConfig {
        let builder = ServerConfig::builder().with_no_client_auth().with_cert_resolver(self.to_arc());
        builder
    }

    pub fn add_cert(&self, name: String, cert: CertifiedKey) -> bool {
        self.sni_match.insert(name, Arc::new(cert)).is_some()
    }

    pub fn select_cert(&self, sni: &str) -> Option<Arc<CertifiedKey>> {
        let sni_lab = sni.split('.').collect::<Vec<&str>>();

        for shard in self.sni_match.iter() {
            let name_lab = shard.key().split('.').collect::<Vec<&str>>();
            
            if name_lab.len() != sni_lab.len() {
                continue;
            }
            
            if name_lab[0] == "*" && name_lab[1..] == sni_lab[1..] {
                return Some(shard.value().clone());
            }
            else if name_lab == sni_lab {
                return Some(shard.value().clone());
            }
        }
        None
    }
}
impl ResolvesServerCert for TlsCertSelector{
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        if let Some(sni) = client_hello.server_name() && let Some(cert) = self.select_cert(sni) {
            Some(cert.clone())
        }
        else if let Some(cert) = &self.default {
            Some(cert.clone())
        }
        else {
            None
        }
    }
}

pub async fn tls_upgrade(tcp: TcpStream, conf: Arc<ServerConfig>) -> io::Result<TlsStream<TcpStream>> {
    let acceptor = TlsAcceptor::from(conf);
    acceptor.accept(tcp).await
}
