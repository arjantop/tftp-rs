//! A Trivial File Transfer (TFTP) protocol client implementation.
//!
//! This module contains the ability to read data from or write data to a remote TFTP server.

use std::convert::From;
use std::io;
use std::path::Path;
use std::net::SocketAddr;
use std::str::FromStr;
use std::result;
use std::error;
use std::fmt;

use packet::{Mode, RequestPacket, DataPacketOctet, AckPacket, ErrorPacket,
             EncodePacket, RawPacket, Opcode};

use mio::udp::UdpSocket;

static MAX_DATA_SIZE: usize = 512;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "IO error: {}", err),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

pub type Result<T> = result::Result<T, Error>;

trait PacketSender {
    fn send_read_request(&self, path: &str, mode: Mode) -> io::Result<()>;
    fn send_ack(&self, block_id: u16) -> io::Result<()>;
}

trait PacketReceiver {
    fn receive_data(&mut self) -> io::Result<DataPacketOctet<'static>>;
}

struct InternalClient {
    socket: UdpSocket,
    remote_addr: SocketAddr,
}

impl InternalClient {
    fn new(socket: UdpSocket, remote_addr: SocketAddr) -> InternalClient {
        InternalClient { socket: socket, remote_addr: remote_addr }
    }
}

impl PacketSender for InternalClient {
    fn send_read_request(&self, path: &str, mode: Mode) -> io::Result<()> {
        let read_request = RequestPacket::read_request(path, mode);
        let encoded = read_request.encode();
        let buf = encoded.packet_buf();
        self.socket.send_to(&buf, &self.remote_addr).map(|_| ())
    }

    fn send_ack(&self, block_id: u16) -> io::Result<()> {
        let ack = AckPacket::new(block_id);
        let encoded = ack.encode();
        let buf = encoded.packet_buf();
        self.socket.send_to(&buf, &self.remote_addr).map(|_| ())
    }
}

impl PacketReceiver for InternalClient {
    fn receive_data(&mut self) -> io::Result<DataPacketOctet<'static>> {
        loop {
            let mut buf = vec![0; MAX_DATA_SIZE + 4];
            let result = match self.socket.recv_from(&mut buf) {
                Ok(Some(result)) => Ok(result),
                Ok(None) => {
                    continue;
                }
                Err(err) => Err(err)
            };
            return result.map(|(n, from)| {
                self.remote_addr = from;
                RawPacket::new(buf, n)
            }).and_then(|packet| {
                match packet.opcode() {
                    Some(Opcode::DATA) => packet.decode::<DataPacketOctet>().ok_or(io::Error::new(io::ErrorKind::Other, "todo")),
                    Some(Opcode::ERROR) => Err(io::Error::new(io::ErrorKind::Other, "error")),
                    _ => Err(io::Error::new(io::ErrorKind::Other, "unexpected"))
                }
            })
        }
    }
}

/// A Trivial File Transfer Protocol client.
pub struct Client {
    c: InternalClient
}

impl Client {
    /// Creates a new client and binds an UDP socket.
    pub fn new(remote_addr: SocketAddr) -> Result<Client> {
        // FIXME: address should not be hardcoded
        let addr = FromStr::from_str("127.0.0.1:0").unwrap();
        let socket = try!(UdpSocket::bound(&addr));
        Ok(Client{ c: InternalClient::new(socket, remote_addr) })
    }

    /// A TFTP read request
    ///
    /// Get a file `path` from the server using a `mode`. Received data is written to
    /// the `writer`.
    pub fn get(&mut self, path: &Path, mode: Mode, writer: &mut io::Write) -> Result<()> {
        try!(self.c.send_read_request(&path.to_string_lossy(), mode));

        let mut current_id = 1;
        loop {
            match self.c.receive_data() {
                Ok(data_packet) => {
                    if current_id == data_packet.block_id() {
                        try!(self.c.send_ack(data_packet.block_id()));

                        try!(writer.write_all(data_packet.data()));
                        if data_packet.data().len() < MAX_DATA_SIZE {
                            println!("Transfer complete");
                            break;
                        }
                        current_id += 1;
                    } else {
                        println!("Unexpected packet id: got={}, expected={}",
                                 data_packet.block_id(), current_id);
                    }
                }
                Err(_) => return Err(From::from(io::Error::new(io::ErrorKind::Other, "todo")))
            }
        }
        return Ok(())
    }
}
