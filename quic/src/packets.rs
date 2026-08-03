

// #[derive(Debug)]
// pub enum Packet<'a> {
//     VersionNegotiation(VersionNegotiation<'a>),
//     Initial(Initial<'a>),
//     ZeroRTT(ZeroRTT<'a>),
//     Handshake(Handshake<'a>),
//     Retry(Retry<'a>),
//     OneRTT(OneRTT<'a>),
// }

#[derive(Debug)]
pub struct VersionNegotiation<'a> {
    pub dcid: &'a [u8],
    pub scid: &'a [u8],
    pub version: u32,
}

#[derive(Debug)]
pub struct Initial<'a> {
    pub version: u32,
    pub dcid: &'a [u8],
    pub scid: &'a [u8],
    pub token: &'a [u8],
    pub packet_number: u32,
    pub payload: &'a [u8],
}

#[derive(Debug)]
pub struct ZeroRTT<'a> {
    pub version: u32,
    pub dcid: &'a [u8],
    pub scid: &'a [u8],
    pub packet_number: u32,
    pub payload: &'a [u8],
}

#[derive(Debug)]
pub struct Handshake<'a> {
    pub version: u32,
    pub dcid: &'a [u8],
    pub scid: &'a [u8],
    pub packet_number: u32,
    pub payload: &'a [u8],
}

#[derive(Debug)]
pub struct Retry<'a> {
    pub version: u32,
    pub dcid: &'a [u8],
    pub scid: &'a [u8],
    pub retry_token: &'a [u8],
    pub integrity_tag: u128,
}

#[derive(Debug)]
pub struct OneRTT<'a> {
    pub spin: bool,
    pub keyphase: bool,
    pub dcid: &'a [u8],
    pub packet_number: u32,
    pub payload: &'a [u8],
}

