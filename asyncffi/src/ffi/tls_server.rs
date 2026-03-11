use std::{ffi::CStr, ptr, sync::Arc};

use httprs_core::ffi::{futures::FfiFuture, slice::FfiSlice};
use rustls::{ServerConfig, pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject}, sign::CertifiedKey};
use tokio_rustls::TlsAcceptor;

use crate::{DynStream, PROVIDER, ffi::utils::heap_void_ptr, servers::TlsCertSelector, spawn_task_with};



#[unsafe(no_mangle)]
pub extern "C" fn tls_config_single_cert_pem(certs: FfiSlice, key: FfiSlice, alpns: *mut i8) -> *const ServerConfig {
    let prov = (*PROVIDER).clone();

    let certs = CertificateDer::pem_reader_iter(certs.as_bytes()).map(|c| c.and_then(|c| Ok(c.into_owned()))).collect::<Result<Vec<_>, _>>();
    let key = PrivateKeyDer::from_pem_reader(key.as_bytes());

    let alpns = unsafe { CStr::from_ptr(alpns).to_string_lossy().to_string() };
    let alpns = alpns.split(',').map(|s|s.as_bytes().to_vec()).collect();

    if 
        let Ok(certs) = certs && 
        let Ok(key) = key && 
        let Ok(build) = ServerConfig::builder_with_provider(prov).with_protocol_versions(rustls::DEFAULT_VERSIONS) && 
        let Ok(mut conf) = build.with_no_client_auth().with_single_cert(certs, key) 
    {
        conf.alpn_protocols = alpns;
        Arc::into_raw(Arc::new(conf))
    }
    else {
        ptr::null()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tls_config_sni_builder() -> *const TlsCertSelector {
    let builder = TlsCertSelector::new();

    Arc::into_raw(builder.to_arc())
}

#[unsafe(no_mangle)]
pub extern "C" fn tls_config_sni_builder_with_pem(def_certs: FfiSlice, def_key: FfiSlice) -> *const TlsCertSelector {
    let certs = if let Ok(certs) = CertificateDer::pem_reader_iter(def_certs.as_bytes()).map(|c| c.and_then(|c| Ok(c.into_owned()))).collect::<Result<Vec<_>, _>>() { certs } else { return ptr::null() };
    let key = if let Ok(key) = PrivateKeyDer::from_pem_reader(def_key.as_bytes()) { key } else { return ptr::null() };
    let cert = if let Ok(cert) = CertifiedKey::from_der(certs, key, &PROVIDER) { cert } else { return ptr::null() };

    let builder = TlsCertSelector::with_default(cert);

    Arc::into_raw(builder.to_arc())
}

#[unsafe(no_mangle)]
pub extern "C" fn tls_config_sni_add_pem(sni_build: *const TlsCertSelector, domain: *mut i8, certs: FfiSlice, key: FfiSlice) -> bool {
    unsafe {
        let domain = CStr::from_ptr(domain).to_string_lossy().to_string();

        let certs = if let Ok(certs) = CertificateDer::pem_reader_iter(certs.as_bytes()).map(|c| c.and_then(|c| Ok(c.into_owned()))).collect::<Result<Vec<_>, _>>() { certs } else { return false };
        let key = if let Ok(key) = PrivateKeyDer::from_pem_reader(key.as_bytes()) { key } else { return false };
        let cert = if let Ok(cert) = CertifiedKey::from_der(certs, key, &PROVIDER) { cert } else { return false };

        (*sni_build).add_cert(domain, cert);
        true
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tls_config_sni_builder_build(sni_build: *const TlsCertSelector, alpns: *mut i8) -> *const ServerConfig {
    unsafe{
        let sni = Arc::from_raw(sni_build);
        let prov = (*PROVIDER).clone();

        let alpns = CStr::from_ptr(alpns).to_string_lossy().to_string();
        let alpns = alpns.split(',').map(|s|s.as_bytes().to_vec()).collect();
        
        if let Ok(build) = ServerConfig::builder_with_provider(prov).with_protocol_versions(rustls::DEFAULT_VERSIONS) {
            let mut conf = build.with_no_client_auth().with_cert_resolver(sni);
            conf.alpn_protocols = alpns;
            
            let conf = Arc::new(conf);
            Arc::into_raw(conf)
        }
        else{
            ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tls_config_free(conf: *const ServerConfig) {
    unsafe {
        drop(Arc::from_raw(conf));
    }
}


#[unsafe(no_mangle)]
pub extern "C" fn tcp_upgrade_tls(fut: *mut FfiFuture, stream: *mut DynStream, conf: *const ServerConfig){
    unsafe {
        let stream = *Box::from_raw(stream);
        let fut = &*fut;
        let con = {
            Arc::increment_strong_count(conf);
            Arc::from_raw(conf)
        };
        let acc = TlsAcceptor::from(con);

        spawn_task_with(fut, async move {
            match stream {
                DynStream::Tcp(tcp) => {
                    let tls = acc.accept(tcp).await?;
                    let stream: DynStream = tls.into();
                    Ok(heap_void_ptr(stream))
                },
                DynStream::Duplex(dup) => {
                    let tls = acc.accept(dup).await?;
                    let stream: DynStream = tls.into();
                    Ok(heap_void_ptr(stream))
                },
                _ => {
                    Ok(heap_void_ptr(stream))
                },
            }
        })
    }
}
