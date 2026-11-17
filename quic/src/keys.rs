use aes::Aes128;
use aes_gcm::Aes128Gcm;
use cipher::KeyInit;
use hkdf::Hkdf;
use sha2::Sha256;

pub const INITIAL_SALT_V1: &'static [u8] = &[0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8, 0x0c, 0xad, 0xcc, 0xbb, 0x7f, 0x0a];

pub const CLIENT_IN: &'static [u8] = &[0x00, 0x20, 0x0f, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x63, 0x6c, 0x69, 0x65, 0x6e, 0x74, 0x20, 0x69, 0x6e, 0x00];
pub const SERVER_IN: &'static [u8] = &[0x00, 0x20, 0x0f, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x20, 0x69, 0x6e, 0x00];

pub const QUIC_KEY: &'static [u8] = &[0x00, 0x10, 0x0e, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x71, 0x75, 0x69, 0x63, 0x20, 0x6b, 0x65, 0x79, 0x00];
pub const QUIC_IV: &'static [u8] = &[0x00, 0x0c, 0x0d, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x71, 0x75, 0x69, 0x63, 0x20, 0x69, 0x76, 0x00];
pub const QUIC_HP: &'static [u8] = &[0x00, 0x10, 0x0d, 0x74, 0x6c, 0x73, 0x31, 0x33, 0x20, 0x71, 0x75, 0x69, 0x63, 0x20, 0x68, 0x70, 0x00];


#[derive(Debug)]
pub struct DerivedKeys {
    // pub initial: sha2::digest::Output<Sha256>,
    
    // pub client_initial: [u8; 32],
    // pub client_key: [u8; 16],
    pub client_iv: [u8; 12],
    // pub client_hp: [u8; 16],
    pub client_key_cipher: Aes128Gcm,
    pub client_hp_cipher: Aes128,

    // pub server_initial: [u8; 32],
    // pub server_key: [u8; 16],
    pub server_iv: [u8; 12],
    // pub server_hp: [u8; 16],
    pub server_key_cipher: Aes128Gcm,
    pub server_hp_cipher: Aes128,
}


// https://www.rfc-editor.org/info/rfc8446/#section-7.1
pub fn hkdf_expand_label(secret: &[u8], label: &[u8], out_len: usize) -> Option<Vec<u8>> {
    // label always start with tls13 RFC 8446 7.1

    let mut hkdf_label = Vec::with_capacity(4 + label.len());
    hkdf_label.extend_from_slice(&(out_len as u16).to_be_bytes());
    hkdf_label.push(label.len() as u8);
    hkdf_label.extend_from_slice(label);
    hkdf_label.push(0);

    let hkdf = Hkdf::<Sha256>::from_prk(secret).ok()?;
    let mut okm = vec![0; out_len];
    hkdf.expand(&hkdf_label, &mut okm).ok()?;

    Some(okm)
}
pub fn hkdf_expand<const S: usize>(secret: &[u8], hkdf_label: &[u8]) -> Option<[u8; S]> {
    let hkdf = Hkdf::<Sha256>::from_prk(secret).ok()?;
    let mut okm = [0; S];
    hkdf.expand(&hkdf_label, &mut okm).ok()?;

    Some(okm)
}

pub fn derive_keys(dcid: &[u8]) -> Option<DerivedKeys> {
    let (initial, _extract) = Hkdf::<Sha256>::extract(Some(INITIAL_SALT_V1), dcid);
    

    let client_initial = hkdf_expand::<32>(&initial, CLIENT_IN)?;
    let client_key = hkdf_expand::<16>(&client_initial, QUIC_KEY)?;
    let client_iv = hkdf_expand::<12>(&client_initial, QUIC_IV)?;
    let client_hp = hkdf_expand::<16>(&client_initial, QUIC_HP)?;

    let server_initial = hkdf_expand::<32>(&initial, SERVER_IN)?;
    let server_key = hkdf_expand::<16>(&server_initial, QUIC_KEY)?;
    let server_iv = hkdf_expand::<12>(&server_initial, QUIC_IV)?;
    let server_hp = hkdf_expand::<16>(&server_initial, QUIC_HP)?;


    let client_key_cipher = Aes128Gcm::new((&client_key).try_into().ok()?);
    let server_key_cipher = Aes128Gcm::new((&server_key).try_into().ok()?);
    
    let client_hp_cipher = Aes128::new((&client_hp).try_into().ok()?);
    let server_hp_cipher = Aes128::new((&server_hp).try_into().ok()?);


    Some(DerivedKeys { 
        // initial, 

        // client_initial, 
        // client_key, 
        client_iv, 
        // client_hp,
        client_hp_cipher,
        client_key_cipher,

        // server_initial, 
        // server_key, 
        server_iv, 
        // server_hp 
        server_hp_cipher,
        server_key_cipher,
    })
}