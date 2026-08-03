#[derive(Debug)]
pub enum Frame {
    Padding,
    Ping,
    Ack(Ack),
    ResetStream(ResetStream),
    StopSending(StopSending),
    Crypto(Crypto),
    NewToken(NewToken),
    Stream(Stream),
    MaxData(MaxData),
    MaxStreamData(MaxStreamData),
    MaxStreams(MaxStreams),
    DataBlocked(DataBlocked),
    StreamDataBlocked(StreamDataBlocked),
    StreamsBlocked(StreamsBlocked),
    NewConnectionId(NewConnectionId),
    RetireConnectionId(RetireConnectionId),
    PathChallenge(PathChallenge),
    PathResponse(PathResponse),
    ConnectionClose(ConnectionClose),
    HandshakeDone(HandshakeDone),
    Datagram(Datagram),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameReaderError {
    EOF,
}

#[derive(Debug)]
pub struct FrameReader<'a> {
    pub buf: &'a [u8],
    pub index: usize,
}
impl<'a> Iterator for FrameReader<'a> {
    type Item = Result<Frame, FrameReaderError>;
    fn next(&mut self) -> Option<Self::Item> {
        todo!("something")
    }
}


#[derive(Debug, Copy, Clone)]
pub struct Padding;

#[derive(Debug, Copy, Clone)]
pub struct Ping;

#[derive(Debug)]
pub struct Ack;

#[derive(Debug)]
pub struct ResetStream;

#[derive(Debug)]
pub struct StopSending;

#[derive(Debug)]
pub struct Crypto;

#[derive(Debug)]
pub struct NewToken;

#[derive(Debug)]
pub struct Stream;

#[derive(Debug)]
pub struct MaxData;

#[derive(Debug)]
pub struct MaxStreamData;

#[derive(Debug)]
pub struct MaxStreams;

#[derive(Debug)]
pub struct DataBlocked;

#[derive(Debug)]
pub struct StreamDataBlocked;

#[derive(Debug)]
pub struct StreamsBlocked;

#[derive(Debug)]
pub struct NewConnectionId;

#[derive(Debug)]
pub struct RetireConnectionId;

#[derive(Debug)]
pub struct PathChallenge;

#[derive(Debug)]
pub struct PathResponse;

#[derive(Debug)]
pub struct ConnectionClose;

#[derive(Debug)]
pub struct HandshakeDone;

#[derive(Debug)]
pub struct Datagram;
