use std::{io, sync::Arc};
use http::extra::PolyHttpRequest;
use rustls::{SignatureScheme, client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier}};
use tokio_rustls::{TlsConnector, rustls::ClientConfig};
use tokio::{io::{ReadHalf, WriteHalf}, net::{TcpStream, ToSocketAddrs}};

use crate::{DynStream, PROVIDER};


// use crate::DynStream;

pub type DynHttpRequest = PolyHttpRequest<ReadHalf<DynStream>, WriteHalf<DynStream>>;

pub async fn tcp_connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
    // not alot going on
    TcpStream::connect(addr).await
}

pub async fn tls_upgrade(tcp: TcpStream, domain: String, alpn: Vec<Vec<u8>>) -> io::Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let prov = (*PROVIDER).clone();

    let mut root = rustls::RootCertStore::empty();
    root.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    
    let mut conf = ClientConfig::builder_with_provider(prov)
        .with_protocol_versions(rustls::DEFAULT_VERSIONS).map_err(|_| io::Error::new(io::ErrorKind::Other, "failed setting versions"))?
        .with_root_certificates(root).with_no_client_auth();
    conf.alpn_protocols = alpn;
    let conf = Arc::new(conf);

    let conn = TlsConnector::from(conf);
    let domain = rustls::pki_types::ServerName::try_from(domain).map_err(|_| io::Error::new(io::ErrorKind::Other, "invalid domain"))?;
    
    let tls: tokio_rustls::client::TlsStream<TcpStream> = conn.connect(domain, tcp).await?;
    Ok(tls)
}

pub async fn tls_upgrade_no_verification(tcp: TcpStream, domain: String, alpn: Vec<Vec<u8>>) -> io::Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let prov = (*PROVIDER).clone();

    let mut conf = ClientConfig::builder_with_provider(prov)
        .with_protocol_versions(rustls::DEFAULT_VERSIONS).map_err(|_| io::Error::new(io::ErrorKind::Other, "failed setting versions"))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertVal))
        .with_no_client_auth();
    conf.alpn_protocols = alpn;
    let conf = Arc::new(conf);

    let conn = TlsConnector::from(conf);
    let domain = rustls::pki_types::ServerName::try_from(domain).map_err(|_| io::Error::new(io::ErrorKind::Other, "invalid domain"))?;
    
    let tls: tokio_rustls::client::TlsStream<TcpStream> = conn.connect(domain, tcp).await?;
    Ok(tls)
}

#[derive(Debug)]
pub struct NoCertVal;
impl ServerCertVerifier for NoCertVal{
    fn verify_server_cert(
            &self,
            _end_entity: &rustls::pki_types::CertificateDer<'_>,
            _intermediates: &[rustls::pki_types::CertificateDer<'_>],
            _server_name: &rustls::pki_types::ServerName<'_>,
            _ocsp_response: &[u8],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
            SignatureScheme::ML_DSA_44,
            SignatureScheme::ML_DSA_65,
            SignatureScheme::ML_DSA_87,
        ]
    }
}
