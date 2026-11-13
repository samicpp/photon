use hkdf::Hkdf;
use sha2::{Sha256, digest::Output};

pub const INITIAL_SALT_V1: &'static [u8] = &[0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8, 0x0c, 0xad, 0xcc, 0xbb, 0x7f, 0x0a];

// pub const CLIENT_IN: &'static [u8] = &[0x00, 0x20, 0x0f, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x63, 0x6c, 0x69, 0x65, 0x6e, 0x74, 0x20, 0x69, 0x6e, 0x00];
// pub const SERVER_IN: &'static [u8] = &[0x00, 0x20, 0x0f, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x20, 0x69, 0x6e, 0x00];

// pub const QUIC_KEY: &'static [u8] = &[0x00, 0x10, 0x0e, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x71, 0x75, 0x69, 0x63, 0x20, 0x6b, 0x65, 0x79, 0x00];
// pub const QUIC_IV: &'static [u8] = &[0x00, 0x0c, 0x0d, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x71, 0x75, 0x69, 0x63, 0x20, 0x69, 0x76, 0x00];
// pub const QUIC_HP: &'static [u8] = &[0x00, 0x10, 0x0d, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x71, 0x75, 0x69, 0x63, 0x20, 0x68, 0x70, 0x00];


#[derive(Debug)]
pub struct DerivedKeys {
    pub initial: Output<Sha256>,
    
    pub client_initial: Vec<u8>,
    pub client_key: Vec<u8>,
    pub client_iv: Vec<u8>,
    pub client_hp: Vec<u8>,

    pub server_initial: Vec<u8>,
    pub server_key: Vec<u8>,
    pub server_iv: Vec<u8>,
    pub server_hp: Vec<u8>,
}


// https://www.rfc-editor.org/info/rfc8446/#section-7.1
pub fn hkdf_expand_label(secret: &[u8], label: &[u8], out_len: usize) -> Option<Vec<u8>> {
    // label always start with tls13 RFC 8446 7.1

    let mut hkdf_label = Vec::with_capacity(3 + label.len());
    hkdf_label.extend_from_slice(&(out_len as u16).to_be_bytes());
    hkdf_label.push(label.len() as u8);
    hkdf_label.extend_from_slice(label);
    hkdf_label.push(0);

    let hkdf = Hkdf::<Sha256>::from_prk(secret).ok()?;
    let mut okm = vec![0; out_len];
    hkdf.expand(&hkdf_label, &mut okm).ok()?;

    Some(okm)
}

pub fn derive_keys(dcid: &[u8]) -> Option<DerivedKeys> {
    let (initial, _extract) = Hkdf::<Sha256>::extract(Some(INITIAL_SALT_V1), dcid);
    

    let client_initial = hkdf_expand_label(&initial, b"tls13 client in", 32)?;
    let client_key = hkdf_expand_label(&client_initial, b"tls13 quic key", 16)?;
    let client_iv = hkdf_expand_label(&client_initial, b"tls13 quic iv", 12)?;
    let client_hp = hkdf_expand_label(&client_initial, b"tls13 quic hp", 16)?;

    let server_initial = hkdf_expand_label(&initial, b"tls13 server in", 32)?;
    let server_key = hkdf_expand_label(&server_initial, b"tls13 quic key", 16)?;
    let server_iv = hkdf_expand_label(&server_initial, b"tls13 quic iv", 12)?;
    let server_hp = hkdf_expand_label(&server_initial, b"tls13 quic hp", 16)?;


    Some(DerivedKeys { initial, client_initial, client_key, client_iv, client_hp, server_initial, server_key, server_iv, server_hp })
}