//! A Trivial File Transfer Protocol (TFTP) packet utilities.

extern crate byteorder;

use std::io::{Write, Cursor};
use std::borrow::Cow;
use std::convert::From;
use std::error;
use std::fmt;
use std::str::{self, FromStr};

use netascii::{NetasciiString, to_netascii, from_netascii};

use self::byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};


/// Opcode that represents packet's type.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Opcode {
    /// Read request
    RRQ   = 1,

    /// Write request
    WRQ   = 2,

    /// Data
    DATA  = 3,

    /// Acknowledgment
    ACK   = 4,

    /// Error
    ERROR = 5,
}

impl Opcode {
    /// Converts an u16 opcode representation to `Opcode`.
    ///
    /// If numeric opcode is invalid `None` is returned.
    pub fn from_u16(opcode: u16) -> Option<Opcode> {
        match opcode {
            1 => Some(Opcode::RRQ),
            2 => Some(Opcode::WRQ),
            3 => Some(Opcode::DATA),
            4 => Some(Opcode::ACK),
            5 => Some(Opcode::ERROR),
            _ => None
        }
    }
}

/// Mode of data transfer
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Mode {
    /// Netascii transfer mode.
    ///
    /// Standard ascii with the modifications from the "Telnet Protocol Specification"
    /// (http://tools.ietf.org/html/rfc764).
    NetAscii,

    /// Octet transfer mode.
    ///
    /// Binary mode, raw 8-bit bytes.
    Octet,
}

impl Mode {
    /// Converts a transfer mode into string representation.
    pub fn as_str(&self) -> &'static str {
        match *self {
            Mode::NetAscii => "netascii",
            Mode::Octet => "octet"
        }
    }
}

#[derive(Debug)]
pub struct ParseModeError;

impl fmt::Display for ParseModeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "provided string was not `netascii` or `octet`".fmt(f)
    }
}

impl error::Error for ParseModeError {
    fn description(&self) -> &str { "failed to parse Mode" }
}

impl FromStr for Mode {
    type Err = ParseModeError;

    fn from_str(s: &str) -> Result<Mode, ParseModeError> {
        match s {
            "netascii" => Ok(Mode::NetAscii),
            "octet" => Ok(Mode::Octet),
            _ => Err(ParseModeError)
        }
    }
}

/// Error codes
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Error {
    /// Not defined, see error message.
    Undefined                 = 0,

    /// File not found.
    FileNotFound              = 1,

    /// Access violation.
    AccessViolation           = 2,

    /// Disk full or allocation exceeded.
    DiskFull                  = 3,

    /// Illegal TFTP operation.
    IllegalOperation          = 4,

    /// Unknown transfer ID.
    UnknownTransferId         = 5,

    /// File already exists.
    FileAlreadyExists         = 6,

    /// No such user
    NoSuchUser                = 7,
}

impl Error {
    /// Converts an u16 erro code to `Error`.
    ///
    /// If numeric error code is invalid `None` is returned.
    fn from_u16(code: u16) -> Option<Error> {
        match code {
            0 => Some(Error::Undefined),
            1 => Some(Error::FileNotFound),
            2 => Some(Error::AccessViolation),
            3 => Some(Error::DiskFull),
            4 => Some(Error::IllegalOperation),
            5 => Some(Error::UnknownTransferId),
            6 => Some(Error::FileAlreadyExists),
            7 => Some(Error::NoSuchUser),
            _ => None
        }
    }
}

impl<'a> fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Undefined => "undefined",
            Error::FileNotFound => "file not found",
            Error::AccessViolation => "access violation",
            Error::DiskFull => "disk full",
            Error::IllegalOperation => "illegal operation",
            Error::UnknownTransferId => "unknown transfer id",
            Error::FileAlreadyExists => "file already exists",
            Error::NoSuchUser => "no such user",
        }.fmt(f)
    }
}

/// A trait to represent common packet data.
pub trait Packet {
    /// Returns opcode value associated with that packet.
    fn opcode(&self) -> Opcode;

    /// Returns number of bytes of the encoded packet.
    fn len(&self) -> usize;
}

/// General packet decoding.
///
/// Decoding should be implemented using slicing when possible, allocating memory
/// only when really required (e.g. raw value must be unescaped).
pub trait DecodePacket<'a> : Sized {
    /// Decode a packet from a given byte slice.
    ///
    /// If the packet can't be decoded `None` is returned.
    #[inline]
    fn decode(&'a [u8]) -> Option<Self>;
}

/// General packet encoding.
pub trait EncodePacket : Packet {
    /// Encode a packet using a newly allocated buffer.
    ///
    /// This method is provided only for convenience, use `encode_using` for
    /// maximal buffer reuse when possible.
    #[inline]
    fn encode(&self) -> RawPacket {
        self.encode_using(vec![0u8; self.len()])
    }

    /// Encode a packet using the the provided buffer.
    #[inline]
    fn encode_using(&self, buf: Vec<u8>) -> RawPacket;
}

/// Request packet
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum RequestPacket<'a> {
    /// Read request packet
    ReadRequest(NetasciiString<'a>, Mode),

    /// Write request packet
    WriteRequest(NetasciiString<'a>, Mode),
}

// FIXME
unsafe impl<'a> Send for RequestPacket<'a> {}

impl<'a> RequestPacket<'a> {
    /// Creates a new read request.
    ///
    /// Filename is converted to netascii if required.
    pub fn read_request<'b>(filename: &'b str, mode: Mode) -> RequestPacket<'b> {
        RequestPacket::ReadRequest(to_netascii(filename), mode)
    }

    /// Create a new write request.
    ///
    /// Filename is converted to netascii if required.
    pub fn write_request<'b>(filename: &'b str, mode: Mode) -> RequestPacket<'b> {
        RequestPacket::WriteRequest(to_netascii(filename), mode)
    }

    /// Returns a file name that the request is for.
    ///
    /// If netascii encoding is invalid `None` is returned.
    pub fn filename<'b>(&'b self) -> Option<Cow<'b, str>> {
        from_netascii(self.filename_raw())
    }

    /// Returns a raw file name netascii encoded.
    pub fn filename_raw(&self) -> &str {
        match *self {
            RequestPacket::ReadRequest(ref filename, _) => &filename[..],
            RequestPacket::WriteRequest(ref filename, _) => &filename[..],
        }
    }

    /// Returns a transfer mode.
    pub fn mode(&self) -> Mode {
        match *self {
            RequestPacket::ReadRequest(_, mode) => mode,
            RequestPacket::WriteRequest(_, mode) => mode
        }
    }
}

impl<'a> Packet for RequestPacket<'a> {
    fn opcode(&self) -> Opcode {
        match *self {
            RequestPacket::ReadRequest(_, _) => Opcode::RRQ,
            RequestPacket::WriteRequest(_, _) => Opcode::WRQ
        }
    }

    fn len(&self) -> usize {
        2 + self.filename_raw().len() + 1 + self.mode().as_str().len() + 1
    }
}

impl<'a> DecodePacket<'a> for RequestPacket<'a> {
    fn decode(data: &'a [u8]) -> Option<RequestPacket<'a>> {
        let mut cur = Cursor::new(data);
        let opcode = cur.read_u16::<BigEndian>().ok().and_then(Opcode::from_u16);

        if opcode != Some(Opcode::RRQ) && opcode != Some(Opcode::WRQ) {
            return None
        }
        // FIXME
        str::from_utf8(&data[2..]).ok().map(|s| s.split('\0')).and_then(|mut parts| {
            let filename = parts.next().map(|s| Cow::from(s));
            let mode = parts.next().and_then(|m| FromStr::from_str(m).ok());
            match (filename, mode) {
                (Some(filename), Some(mode)) => {
                    if opcode.unwrap() == Opcode::RRQ {
                        Some(RequestPacket::ReadRequest(filename, mode))
                    } else {
                        Some(RequestPacket::WriteRequest(filename, mode))
                    }
                }
                _ => None
            }
        })
    }
}

impl<'a> EncodePacket for RequestPacket<'a> {
    fn encode_using(&self, buf: Vec<u8>) -> RawPacket {
        let mut b = Cursor::new(buf);
        b.write_u16::<BigEndian>(self.opcode() as u16).unwrap();
        b.write(self.filename_raw().as_bytes()).unwrap();
        b.write_u8(0).unwrap();
        b.write(self.mode().as_str().as_bytes()).unwrap();
        b.write_u8(0).unwrap();

        RawPacket {
            buf: b.into_inner(),
            len: self.len()
        }
    }
}

/// Data packet acknowledgment
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct AckPacket {
    block_id: u16,
}

// FIXME
unsafe impl Send for AckPacket {}

impl AckPacket {
    /// Creates a new acknowledgment package for data block with number `block_id`.
    pub fn new(block_id: u16) -> AckPacket {
        AckPacket{
            block_id: block_id
        }
    }

    /// Returns the block number that this acknowledgment is for.
    pub fn block_id(&self) -> u16 {
        self.block_id
    }
}

impl Packet for AckPacket {
    fn opcode(&self) -> Opcode {
        Opcode::ACK
    }

    fn len(&self) -> usize { 4 }
}

impl<'a> DecodePacket<'a> for AckPacket {
    fn decode(data: &'a [u8]) -> Option<AckPacket> {
        let mut cur = Cursor::new(data);
        let opcode = cur.read_u16::<BigEndian>().ok().and_then(Opcode::from_u16);
        match opcode {
            Some(Opcode::ACK) => cur.read_u16::<BigEndian>().ok().map(AckPacket::new),
            _ => None
        }
    }
}

impl EncodePacket for AckPacket {
    fn encode_using(&self, buf: Vec<u8>) -> RawPacket {
        let mut b = Cursor::new(buf);
        b.write_u16::<BigEndian>(Opcode::ACK as u16).unwrap();
        b.write_u16::<BigEndian>(self.block_id).unwrap();

        RawPacket{
            buf: b.into_inner(),
            len: self.len()
        }
    }
}

/// Data packet using octet encoding
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct DataPacketOctet<'a> {
    block_id: u16,
    data: Cow<'a, [u8]>,
    len: usize,
}

// FIXME
unsafe impl<'a> Send for DataPacketOctet<'a> {}

impl<'a> DataPacketOctet<'a> {
    /// Creates a data packet with a given id from provided slice od bytes.
    pub fn from_slice(block_id: u16, data: &[u8]) -> DataPacketOctet {
        DataPacketOctet{
            block_id: block_id,
            data: Cow::from(data),
            len: data.len()
        }
    }

    /// Creates a data packet with a given id from a given vector.
    pub fn from_vec(block_id: u16, data: Vec<u8>, len: usize) -> DataPacketOctet<'static> {
        DataPacketOctet{
            block_id: block_id,
            data: Cow::from(data),
            len: len
        }
    }

    /// Returns block number of this data packet.
    pub fn block_id(&self) -> u16 {
        self.block_id
    }

    /// Returns the slice of bytes contained in this packet.
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }

    /// Tries to move the buffer out of this object and returns it, consuming the `RawPacket`.
    ///
    /// Returns `None` if contained buffer is a slice.
    pub fn get_buffer(self) -> Option<Vec<u8>> {
        match self.data {
            Cow::Owned(v) => Some(v),
            _ => None
        }
    }
}

impl<'a> Packet for DataPacketOctet<'a> {
    fn opcode(&self) -> Opcode {
        Opcode::DATA
    }

    fn len(&self) -> usize {
        4 + self.data.len()
    }
}

impl<'a> DecodePacket<'a> for DataPacketOctet<'static> {
    fn decode(data: &'a [u8]) -> Option<DataPacketOctet<'static>> {
        let mut cur = Cursor::new(data);
        let opcode = cur.read_u16::<BigEndian>().ok().and_then(Opcode::from_u16);
        match opcode {
            Some(Opcode::DATA) => {
                cur.read_u16::<BigEndian>().ok().map(|block_id| {
                    let payload = data[4..].to_vec();
                    let len = payload.len();
                    DataPacketOctet::from_vec(block_id, payload, len)
                })
            }
            _ => None
        }
    }
}

impl<'a> EncodePacket for DataPacketOctet<'a> {
    fn encode_using(&self, buf: Vec<u8>) -> RawPacket {
        let mut b = Cursor::new(buf);
        b.write_u16::<BigEndian>(Opcode::DATA as u16).unwrap();
        b.write_u16::<BigEndian>(self.block_id).unwrap();
        b.write(&self.data[..]).unwrap();

        RawPacket {
            buf: b.into_inner(),
            len: self.len()
        }
    }
}

/// Packet representing an error
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ErrorPacket<'a> {
    error: Error,
    message: NetasciiString<'a>,
}

// FIXME
unsafe impl<'a> Send for ErrorPacket<'a> {}

impl<'a> fmt::Display for ErrorPacket<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

impl<'a> error::Error for ErrorPacket<'a> {
    fn description(&self) -> &str {
        &self.message
    }
}

impl<'a> ErrorPacket<'a> {
    /// Creates and error packet with a chosen error and a message describing the
    /// cause of the error.
    pub fn new(error: Error, msg: &'a str) -> ErrorPacket<'a> {
        ErrorPacket{
            error: error,
            message: to_netascii(msg)
        }
    }

    pub fn error(&self) -> Error {
        self.error
    }

    pub fn message(&'a self) -> Option<Cow<'a, str>> {
        from_netascii(&self.message[..])
    }
}

impl<'a> Packet for ErrorPacket<'a> {
    fn opcode(&self) -> Opcode {
        Opcode::ERROR
    }

    fn len(&self) -> usize {
        4 + self.message.len() + 1
    }
}

impl<'a> DecodePacket<'a> for ErrorPacket<'a> {
    fn decode(data: &'a [u8]) -> Option<ErrorPacket<'a>> {
        let mut cur = Cursor::new(data);
        let opcode = cur.read_u16::<BigEndian>().ok().and_then(Opcode::from_u16);
        match opcode {
            Some(Opcode::ERROR) => {
                let error = cur.read_u16::<BigEndian>().ok().and_then(Error::from_u16);
                // FIXME
                let msg = str::from_utf8(&data[4..]).ok().map(|s| s.split('\0'))
                                                            .and_then(|mut i| i.next());
                match (error, msg) {
                    (Some(error), Some(msg)) => Some(ErrorPacket::new(error, msg)),
                    _ => None
                }
            }
            _ => None
        }
    }
}

impl<'a> EncodePacket for ErrorPacket<'a> {
    fn encode_using(&self, buf: Vec<u8>) -> RawPacket {
        let mut b = Cursor::new(buf);
        b.write_u16::<BigEndian>(Opcode::ERROR as u16).unwrap();
        b.write_u16::<BigEndian>(self.error  as u16).unwrap();
        b.write(&self.message.as_bytes()).unwrap();
        b.write_u8(0).unwrap();

        RawPacket{
            buf: b.into_inner(),
            len: self.len()
        }
    }
}

/// A Trivial File Transfer Protocol encoded packet.
#[derive(Clone)]
pub struct RawPacket {
    buf: Vec<u8>,
    len: usize,
}

impl RawPacket {
    /// Creates a raw TFTP packet from the given buffer with actual data taking
    /// `len` bytes.
    pub fn new(buf: Vec<u8>, len: usize) -> RawPacket {
        RawPacket{
            buf: buf,
            len: len
        }
    }

    /// Returns a slice of bytes representing a packet.
    pub fn packet_buf(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// Returns opcode of an endoded packet.
    ///
    /// Return `None` if read opcode value is unknown.
    pub fn opcode(&self) -> Option<Opcode> {
        self.packet_buf().read_u16::<BigEndian>().ok().and_then(Opcode::from_u16)
    }

    /// Decode a packet of specified type.
    ///
    /// Returns `None` if the packet can't be decoded to a required type.
    pub fn decode<'a, P: Packet + DecodePacket<'a>>(&'a self) -> Option<P> {
        DecodePacket::decode(self.packet_buf())
    }

    /// Length of the encoded packet.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Moves the buffer out of this object and returns it, consuming the `RawPacket`.
    ///
    /// This method should be used for maximal buffer reuse. Memory is zeroed before returning.
    pub fn get_buffer(self) -> Vec<u8> {
        let mut buffer = self.buf;
        // TODO: optimize
        for x in buffer.iter_mut() {
            *x = 0 ;
        }
        buffer
    }
}

#[cfg(test)]
mod test {
    extern crate quickcheck;
    extern crate rand;

    use std::borrow::Cow;
    use std::convert::From;

    use self::rand::Rng;
    use self::quickcheck::{quickcheck, Arbitrary, Gen};

    use super::{Mode, Error, EncodePacket, DecodePacket};
    use super::{RequestPacket, AckPacket, DataPacketOctet,
                ErrorPacket};

    impl Arbitrary for RequestPacket<'static> {
        fn arbitrary<G: Gen>(g: &mut G) -> RequestPacket<'static> {
            let transfer_type = if g.gen() { Mode::Octet } else { Mode::NetAscii };
            let str_len = g.gen_range(0usize, 50);
            let filename: String = g.gen_ascii_chars().take(str_len).collect();
            if g.gen() {
                RequestPacket::ReadRequest(Cow::from(filename), transfer_type)
            } else {
                RequestPacket::WriteRequest(Cow::from(filename), transfer_type)
            }
        }
    }

    impl Arbitrary for AckPacket {
        fn arbitrary<G: Gen>(g: &mut G) -> AckPacket {
            AckPacket::new(g.gen())
        }
    }

    impl Arbitrary for DataPacketOctet<'static> {
        fn arbitrary<G: Gen>(g: &mut G) -> DataPacketOctet<'static> {
            let size = g.gen_range(0usize, 512);
            let data: Vec<_> = g.gen_iter::<u8>().take(size).collect();
            let len = data.len();
            DataPacketOctet::from_vec(g.gen(), data, len)
        }
    }

    impl Arbitrary for ErrorPacket<'static> {
        fn arbitrary<G: Gen>(g: &mut G) -> ErrorPacket<'static> {
            let error = Error::from_u16(g.gen_range(0, 8)).unwrap();
            let msg_len = g.gen_range(0usize, 50);
            let message: String = g.gen_ascii_chars().take(msg_len).collect();
            ErrorPacket{
                error: error,
                message: Cow::from(message)
            }
        }
    }

    #[test]
    fn packet_read_request_with_escape_is_encoded() {
        let packet = RequestPacket::read_request("foo", Mode::Octet);
        let raw_packet = packet.encode();
        let expected = b"\x00\x01foo\0octet\0";
        assert_eq!(expected, raw_packet.packet_buf());
    }

    #[test]
    fn packet_read_request_without_escape_is_encoded() {
        let packet = RequestPacket::read_request("foo\nbar", Mode::Octet);
        let raw_packet = packet.encode();
        let expected = b"\x00\x01foo\r\nbar\0octet\0";
        assert_eq!(expected, raw_packet.packet_buf());
    }

    #[test]
    fn packet_write_request_with_escape_is_encoded() {
        let packet = RequestPacket::write_request("foo", Mode::Octet);
        let raw_packet = packet.encode();
        let expected = b"\x00\x02foo\0octet\0";
        assert_eq!(expected, raw_packet.packet_buf());
    }

    #[test]
    fn packet_write_request_without_escape_is_encoded() {
        let packet = RequestPacket::write_request("foo\nbar", Mode::Octet);
        let raw_packet = packet.encode();
        let expected = b"\x00\x02foo\r\nbar\0octet\0";
        assert_eq!(expected, raw_packet.packet_buf());
    }

    #[test]
    fn request_packet_with_netascii_mode_is_encoded() {
        let packet = RequestPacket::read_request("na", Mode::NetAscii);
        let raw_packet = packet.encode();
        let expected = b"\x00\x01na\0netascii\0";
        assert_eq!(expected, raw_packet.packet_buf());
    }

    #[test]
    fn encoding_and_decoding_request_packet_is_identity() {
        fn prop(packet: RequestPacket<'static>)  -> bool {
            Some(packet.clone()) == packet.encode().decode()
        }
        quickcheck(prop as fn(RequestPacket<'static>) -> bool)
    }

    #[test]
    fn packet_ack_is_encoded() {
        let packet = AckPacket::new(1);
        let raw_packet = packet.encode();
        let expected = vec![0, 4, 0, 1];
        assert_eq!(&expected[..], raw_packet.packet_buf());
    }

    #[test]
    fn encoding_and_decoding_packet_ack_is_identity() {
        fn prop(packet: AckPacket) -> bool {
            Some(packet) == packet.encode().decode()
        }
        quickcheck(prop as fn(AckPacket) -> bool)
    }

    #[test]
    fn packet_data_octet_is_encoded() {
        let packet = DataPacketOctet::from_vec(10, vec![1u8, 2, 3, 4, 5], 5);
        let raw_packet = packet.encode();
        let expected = vec![0, 3, 0, 10, 1, 2, 3, 4, 5];
        assert_eq!(&expected[..], raw_packet.packet_buf());
    }

    #[test]
    fn encoding_and_decoding_packet_data_octet_is_identity() {
        fn prop(packet: DataPacketOctet<'static>) -> bool {
            Some(packet.clone()) == packet.encode().decode()
        }
        quickcheck(prop as fn(DataPacketOctet<'static>) -> bool)
    }

    #[test]
    fn packet_error_is_encoded() {
        let packet = ErrorPacket::new(Error::FileNotFound, "message");
        let raw_packet = packet.encode();
        let expected = b"\x00\x05\x00\x01message\x00";
        assert_eq!(expected, raw_packet.packet_buf())
    }

    #[test]
    fn packet_error_with_netascii_is_encoded() {
        let packet = ErrorPacket::new(Error::DiskFull, "me\rssage\n");
        let raw_packet = packet.encode();
        let expected = b"\x00\x05\x00\x03me\r\0ssage\r\n\x00";
        assert_eq!(expected, raw_packet.packet_buf())
    }

    #[test]
    fn encoding_and_decoding_packet_error_is_identity() {
        fn prop(packet: ErrorPacket<'static>) -> bool {
            Some(packet.clone()) == packet.encode().decode()
        }
        quickcheck(prop as fn(ErrorPacket<'static>) -> bool)
    }

    #[test]
    fn packet_buffer_is_zeroes_before_reuse() {
        let packet = AckPacket::new(1);
        let raw_packet = packet.encode();
        let expected = vec![0; 4];
        assert_eq!(expected, raw_packet.get_buffer());
    }
}

#[cfg(test)]
mod bench {
    extern crate test;

    use self::test::{Bencher, black_box};

    use super::{Mode, EncodePacket, Error};
    use super::{RequestPacket, AckPacket, DataPacketOctet, ErrorPacket};

    #[bench]
    fn decode_read_request(b: &mut Bencher) {
        let raw_packet = RequestPacket::read_request("file", Mode::Octet).encode();
        b.iter(|| {
            let packet: Option<RequestPacket> = raw_packet.decode();
            black_box(packet)
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn encode_read_request(b: &mut Bencher) {
        let packet = RequestPacket::read_request("file", Mode::Octet);
        let raw_packet = packet.encode();
        b.iter(|| {
            black_box(packet.encode())
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn decode_ack(b: &mut Bencher) {
        let raw_packet = AckPacket::new(1).encode();
        b.iter(|| {
            let ack: Option<AckPacket> = raw_packet.decode();
            black_box(ack)
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn encode_ack(b: &mut Bencher) {
        let packet = AckPacket::new(1);
        let raw_packet = packet.encode();
        b.iter(|| {
            black_box(packet.encode())
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn decode_data_octet(b: &mut Bencher) {
        let data = vec![1u8; 100];
        let raw_packet = DataPacketOctet::from_slice(1, &data[..]).encode();
        b.iter(|| {
            let ack: Option<DataPacketOctet> = raw_packet.decode();
            black_box(ack)
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn encode_data_octet(b: &mut Bencher) {
        let data = vec![1u8; 100];
        let packet = DataPacketOctet::from_slice(1, &data[..]);
        let raw_packet = packet.encode();
        b.iter(|| {
            black_box(packet.encode())
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn encode_data_octet_buffer_reusing(b: &mut Bencher) {
        static N: usize = 1000;
        let data = vec![1u8; 100];
        let packet = DataPacketOctet::from_slice(1, &data[..]);
        let raw_packet = packet.encode();

        b.bench_n(N as u64, |b: &mut Bencher| {
            let mut buf = vec!(0u8; 512);
            for _ in 0..N {
                let encoded = packet.encode_using(buf);
                buf = encoded.get_buffer();
            }
            b.bytes = (raw_packet.len() * N) as u64;
        });
    }

    #[bench]
    fn decode_error(b: &mut Bencher) {
        let message = "This is some error message";
        let raw_packet = ErrorPacket::new(Error::FileNotFound, message).encode();
        b.iter(|| {
            let ack: Option<DataPacketOctet> = raw_packet.decode();
            black_box(ack)
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn encode_error(b: &mut Bencher) {
        let message = "This is some error message";
        let packet = ErrorPacket::new(Error::FileNotFound, message);
        let raw_packet = packet.encode();
        b.iter(|| {
            black_box(packet.encode())
        });
        b.bytes = raw_packet.len() as u64;
    }
}
