pub mod methods{
    pub const UNKNOWN: u8 = 0;
    pub const GET: u8 = 1;
    pub const HEAD: u8 = 2;
    pub const POST: u8 = 3;
    pub const PUT: u8 = 4;
    pub const DELETE: u8 = 5;
    pub const CONNECT: u8 = 6;
    pub const OPTIONS: u8 = 7;
    pub const TRACE: u8 = 8;
}
pub mod versions{
    pub const UNKNOWN: u8 = 0;
    pub const DEBUG: u8 = 1;
    pub const HTTP09: u8 = 2;
    pub const HTTP10: u8 = 3;
    pub const HTTP11: u8 = 4;
    pub const HTTP2: u8 = 5;
    pub const HTTP3: u8 = 6;
}
