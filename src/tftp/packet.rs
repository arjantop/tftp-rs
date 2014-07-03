//! A Trivial File Transfer Protocol (TFTP) packet utilities.
extern crate graphviz;

use std::fmt;
use std::from_str::FromStr;
use std::str;
use std::str::MaybeOwned;
use std::io::BufWriter;

use self::graphviz::maybe_owned_vec::{MaybeOwnedVector, Growable, IntoMaybeOwnedVector};

use netascii::{NetasciiString, to_netascii, from_netascii};

/// Opcode that represents packet's type.
#[deriving(Show, Eq, PartialEq, Clone)]
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
            1 => Some(RRQ),
            2 => Some(WRQ),
            3 => Some(DATA),
            4 => Some(ACK),
            5 => Some(ERROR),
            _ => None
        }
    }
}

/// Mode of data transfer
#[deriving(Eq, PartialEq, Clone)]
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
            NetAscii => "netascii",
            Octet => "octet"
        }
    }
}

impl fmt::Show for Mode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write(self.as_str().as_bytes())
    }
}

impl FromStr for Mode {
    fn from_str(s: &str) -> Option<Mode> {
        match s {
            "netascii" => Some(NetAscii),
            "octet" => Some(Octet),
            _ => None
        }
    }
}

/// Error codes
#[deriving(Show, Eq, PartialEq, Clone)]
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
            0 => Some(Undefined),
            1 => Some(FileNotFound),
            2 => Some(AccessViolation),
            3 => Some(DiskFull),
            4 => Some(IllegalOperation),
            5 => Some(UnknownTransferId),
            6 => Some(FileAlreadyExists),
            7 => Some(NoSuchUser),
            _ => None
        }
    }
}

/// A trait to represent common packet data.
pub trait Packet {
    /// Returns opcode value associated with that packet.
    fn opcode(&self) -> Opcode;

    /// Returns number of bytes of the encoded packet.
    fn len(&self) -> uint;
}

/// General packet decoding.
///
/// Decoding should be implemented using slicing when possible, allocating memory
/// only when really required (e.g. raw value must be unescaped).
pub trait DecodePacket<'a> {
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
    /// This method is provided onnly for convenience, use `encode_using` for
    /// maximal buffer reuse when possible.
    #[inline]
    fn encode(&self) -> RawPacket {
        self.encode_using(Vec::from_elem(self.len(), 0u8))
    }

    /// Encode a packet using the the provided buffer.
    #[inline]
    fn encode_using(&self, buf: Vec<u8>) -> RawPacket;
}

/// Request packet
#[deriving(Show, Eq, PartialEq, Clone)]
pub enum RequestPacket<'a> {
    /// Read request packet
    ReadRequest(NetasciiString<'a>, Mode),

    /// Write request packet
    WriteRequest(NetasciiString<'a>, Mode),
}

impl<'a> RequestPacket<'a> {
    /// Creates a new read request.
    ///
    /// Filename is converted to netascii if required.
    pub fn read_request<'a>(filename: &'a str, mode: Mode) -> RequestPacket<'a> {
        ReadRequest(to_netascii(filename), mode)
    }

    /// Create a new write request.
    ///
    /// Filename is converted to netascii if required.
    pub fn write_request<'a>(filename: &'a str, mode: Mode) -> RequestPacket<'a> {
        WriteRequest(to_netascii(filename), mode)
    }

    /// Returns a file name that the request is for.
    ///
    /// If netascii encoding is invalid `None` is returned.
    pub fn filename<'a>(&'a self) -> Option<MaybeOwned<'a>> {
        from_netascii(self.filename_raw())
    }

    /// Returns a raw file name netascii encoded.
    pub fn filename_raw<'a>(&'a self) -> &'a str {
        match *self {
            ReadRequest(ref filename, _) => filename.as_slice(),
            WriteRequest(ref filename, _) => filename.as_slice()
        }
    }

    /// Returns a transfer mode.
    pub fn mode(&self) -> Mode {
        match *self {
            ReadRequest(_, mode) => mode,
            WriteRequest(_, mode) => mode
        }
    }
}

impl<'a> Packet for RequestPacket<'a> {
    fn opcode(&self) -> Opcode {
        match *self {
            ReadRequest(_, _) => RRQ,
            WriteRequest(_, _) => WRQ
        }
    }

    fn len(&self) -> uint {
        2 + self.filename_raw().len() + 1 + self.mode().as_str().len() + 1
    }
}

impl<'a> DecodePacket<'a> for RequestPacket<'a> {
    fn decode(data: &'a [u8]) -> Option<RequestPacket<'a>> {
        let opcode = read_be_u16(data).and_then(Opcode::from_u16);
        if opcode != Some(RRQ) || opcode != Some(WRQ) {
            return None
        }
        str::from_utf8(data.slice_from(2)).map(|s| s.split('\0')).and_then(|mut parts| {
            let filename = parts.next().map(|s| s.into_maybe_owned());
            let mode = parts.next().and_then(FromStr::from_str);
            match (filename, mode) {
                (Some(filename), Some(mode)) => {
                    if opcode.unwrap() == RRQ {
                        Some(ReadRequest(filename, mode))
                    } else {
                        Some(WriteRequest(filename, mode))
                    }
                }
                _ => None
            }
        })
    }
}

impl<'a> EncodePacket for RequestPacket<'a> {
    fn encode_using(&self, mut buf: Vec<u8>) -> RawPacket {
        {
            let mut w = BufWriter::new(buf.as_mut_slice());
            w.write_be_u16(self.opcode() as u16);
            w.write_str(self.filename_raw());
            w.write_u8(0);
            w.write_str(self.mode().as_str());
            w.write_u8(0);
        }
        RawPacket{
            buf: buf,
            len: self.len()
        }
    }
}

/// Data packet acknowledgment
#[deriving(Show, Eq, PartialEq, Clone)]
pub struct AckPacket {
    block_id: u16,
}

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
        ACK
    }

    fn len(&self) -> uint { 4 }
}

impl<'a> DecodePacket<'a> for AckPacket {
    fn decode(data: &'a [u8]) -> Option<AckPacket> {
        let opcode = read_be_u16(data).and_then(Opcode::from_u16);
        match opcode {
            Some(ACK) => read_be_u16(data.slice_from(2)).map(AckPacket::new),
            _ => None
        }
    }
}

impl EncodePacket for AckPacket {
    fn encode_using(&self, mut buf: Vec<u8>) -> RawPacket {
        {
            write_be_u16(buf.as_mut_slice(), ACK as u16);
            write_be_u16(buf.as_mut_slice().mut_slice_from(2), self.block_id);
        }
        RawPacket{
            buf: buf,
            len: self.len()
        }
    }
}

/// Data packet using octet encoding
#[deriving(Show, Eq, PartialEq, Clone)]
pub struct DataPacketOctet<'a> {
    block_id: u16,
    data: MaybeOwnedVector<'a, u8>,
    len: uint,
}

impl<'a> DataPacketOctet<'a> {
    /// Creates a data packet with a given id from provided slice od bytes.
    pub fn from_slice<'a>(block_id: u16, data: &'a [u8]) -> DataPacketOctet<'a> {
        DataPacketOctet{
            block_id: block_id,
            data: data.into_maybe_owned(),
            len: data.len()
        }
    }

    /// Creates a data packet with a given id from a given vector.
    pub fn from_vec(block_id: u16, data: Vec<u8>, len: uint) -> DataPacketOctet<'static> {
        DataPacketOctet{
            block_id: block_id,
            data: data.into_maybe_owned(),
            len: len
        }
    }

    /// Returns block number of this data packet.
    pub fn block_id(&self) -> u16 {
        self.block_id
    }

    /// Returns the slice of bytes contained in this packet.
    pub fn data<'a>(&'a self) -> &'a [u8] {
        self.data.as_slice().slice_to(self.len)
    }

    /// Tries to move the buffer out of this object and returns it, consuming the `RawPacket`.
    ///
    /// Returns `None` if contained buffer is a slice.
    pub fn get_buffer(self) -> Option<Vec<u8>> {
        match self.data {
            Growable(v) => Some(v),
            _ => None
        }
    }
}

impl<'a> Packet for DataPacketOctet<'a> {
    fn opcode(&self) -> Opcode {
        DATA
    }

    fn len(&self) -> uint {
        4 + self.data.as_slice().len()
    }
}

impl<'a> DecodePacket<'a> for DataPacketOctet<'a> {
    fn decode(data: &'a [u8]) -> Option<DataPacketOctet<'a>> {
        let opcode = read_be_u16(data).and_then(Opcode::from_u16);
        match opcode {
            Some(DATA) => {
                read_be_u16(data.slice_from(2)).map(|block_id| {
                    DataPacketOctet::from_slice(block_id, data.slice_from(4))
                })
            }
            _ => None
        }
    }
}

impl<'a> EncodePacket for DataPacketOctet<'a> {
    fn encode_using(&self, mut buf: Vec<u8>) -> RawPacket {
        {
            let mut w = BufWriter::new(buf.as_mut_slice());
            w.write_be_u16(DATA as u16);
            w.write_be_u16(self.block_id);
            w.write(self.data.as_slice());
        }
        RawPacket{
            buf: buf,
            len: self.len()
        }
    }
}

/// A Trivial File Transfer Protocol encoded packet.
pub struct RawPacket {
    buf: Vec<u8>,
    len: uint,
}

impl RawPacket {
    /// Creates a raw TFTP packet from the given buffer with actual data taking
    /// `len` bytes.
    pub fn new(buf: Vec<u8>, len: uint) -> RawPacket {
        RawPacket{
            buf: buf,
            len: len
        }
    }

    /// Returns a slice of bytes representing a packet.
    pub fn packet_buf<'a>(&'a self) -> &'a [u8] {
        self.buf.slice_to(self.len)
    }

    /// Returns opcode of an endoded packet.
    ///
    /// Return `None` if read opcode value is unknown.
    pub fn opcode(&self) -> Option<Opcode> {
        read_be_u16(self.packet_buf()).and_then(Opcode::from_u16)
    }

    /// Decode a packet of specified type.
    ///
    /// Returns `None` if the packet can't be decoded to a required type.
    pub fn decode<'a, P: Packet + DecodePacket<'a>>(&'a self) -> Option<P> {
        DecodePacket::decode(self.packet_buf())
    }

    /// Length of the encoded packet.
    pub fn len(&self) -> uint {
        self.len
    }

    /// Moves the buffer out of this object and returns it, consuming the `RawPacket`.
    ///
    /// This method should be used for maximal buffer reuse.
    pub fn get_buffer(self) -> Vec<u8> {
        self.buf
    }
}

fn read_be_u16(data: &[u8]) -> Option<u16> {
    match (data.get(0), data.get(1)) {
        (Some(x1), Some(x2)) => Some(*x1 as u16 << 8 | *x2 as u16),
        _ => None
    }
}

fn write_be_u16(data: &mut [u8], x: u16) -> Option<()> {
    if data.len() < 2 {
        return None
    }
    *data.get_mut(0).unwrap() = (x >> 8) as u8;
    *data.get_mut(1).unwrap() = x as u8;
    Some(())
}

#[cfg(test)]
mod test {
    extern crate quickcheck;

    use std::rand::Rng;

    use self::quickcheck::{quickcheck, Arbitrary, Gen};

    use super::{Octet, EncodePacket, DecodePacket};
    use super::{RequestPacket, AckPacket, DataPacketOctet};

    impl Arbitrary for AckPacket {
        fn arbitrary<G: Gen>(g: &mut G) -> AckPacket {
            AckPacket::new(g.gen())
        }
    }

    impl Arbitrary for DataPacketOctet<'static> {
        fn arbitrary<G: Gen>(g: &mut G) -> DataPacketOctet {
            let size = g.gen_range(0u, 512);
            let data: Vec<_> = g.gen_iter::<u8>().take(size).collect();
            let len = data.len();
            DataPacketOctet::from_vec(g.gen(), data, len)
        }
    }

    #[test]
    fn packet_read_request_with_escape_is_encoded() {
        let packet = RequestPacket::read_request("foo", Octet);
        let raw_packet = packet.encode();
        let expected = b"\x00\x01foo\0octet\0";
        assert_eq!(expected, raw_packet.packet_buf());
    }

    #[test]
    fn packet_read_request_without_escape_is_encoded() {
        let packet = RequestPacket::read_request("foo\nbar", Octet);
        let raw_packet = packet.encode();
        let expected = b"\x00\x01foo\r\nbar\0octet\0";
        assert_eq!(expected, raw_packet.packet_buf());
    }

    #[test]
    fn packet_write_request_with_escape_is_encoded() {
        let packet = RequestPacket::write_request("foo", Octet);
        let raw_packet = packet.encode();
        let expected = b"\x00\x02foo\0octet\0";
        assert_eq!(expected, raw_packet.packet_buf());
    }

    #[test]
    fn packet_write_request_without_escape_is_encoded() {
        let packet = RequestPacket::write_request("foo\nbar", Octet);
        let raw_packet = packet.encode();
        let expected = b"\x00\x02foo\r\nbar\0octet\0";
        assert_eq!(expected, raw_packet.packet_buf());
    }

    #[test]
    fn packet_ack_is_encoded() {
        let packet = AckPacket::new(1);
        let raw_packet = packet.encode();
        let expected = vec![0, 4, 0, 1];
        assert_eq!(expected.as_slice(), raw_packet.packet_buf());
    }

    #[test]
    fn encoding_and_decoding_packet_ack_is_identity() {
        fn prop(packet: AckPacket) -> bool {
            Some(packet) == packet.encode().decode()
        }
        quickcheck(prop)
    }

    #[test]
    fn packet_data_octet_is_encoded() {
        let packet = DataPacketOctet::from_vec(10, vec![1u8, 2, 3, 4, 5], 5);
        let raw_packet = packet.encode();
        let expected = vec![0, 3, 0, 10, 1, 2, 3, 4, 5];
        assert_eq!(expected.as_slice(), raw_packet.packet_buf());
    }

    #[test]
    fn encoding_and_decoding_packet_data_octet_is_identity() {
        fn prop(packet: DataPacketOctet<'static>) -> bool {
            Some(packet.clone()) == packet.encode().decode()
        }
        quickcheck(prop)
    }
}

#[cfg(test)]
mod bench {
    extern crate test;

    use self::test::{Bencher, black_box};

    use super::{Octet, EncodePacket};
    use super::{RequestPacket, AckPacket, DataPacketOctet};

    #[bench]
    fn decode_read_request(b: &mut Bencher) {
        let raw_packet = RequestPacket::read_request("file", Octet).encode();
        b.iter(|| {
            let packet: Option<RequestPacket> = raw_packet.decode();
            black_box(packet)
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn encode_read_request(b: &mut Bencher) {
        let packet = RequestPacket::read_request("file", Octet);
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
        let data = Vec::from_elem(100, 1u8);
        let raw_packet = DataPacketOctet::from_slice(1, data.as_slice()).encode();
        b.iter(|| {
            let ack: Option<DataPacketOctet> = raw_packet.decode();
            black_box(ack)
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn encode_data_octet(b: &mut Bencher) {
        let data = Vec::from_elem(100, 1u8);
        let packet = DataPacketOctet::from_slice(1, data.as_slice());
        let raw_packet = packet.encode();
        b.iter(|| {
            black_box(packet.encode())
        });
        b.bytes = raw_packet.len() as u64;
    }

    #[bench]
    fn encode_data_octet_buffer_reusing(b: &mut Bencher) {
        static N: uint = 1000;
        let data = Vec::from_elem(100, 1u8);
        let packet = DataPacketOctet::from_slice(1, data.as_slice());
        let raw_packet = packet.encode();
        b.iter(|| {
            let mut buf = Vec::from_elem(512, 0u8);
            for _ in range(0, N) {
                let encoded = packet.encode_using(buf);
                buf = encoded.get_buffer();
            }
        });
        b.bytes = (raw_packet.len() * N) as u64;
    }
}
