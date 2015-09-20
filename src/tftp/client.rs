//! A Trivial File Transfer (TFTP) protocol client implementation.
//!
//! This module contains the ability to read data from or write data to a remote TFTP server.

use std::io::{self, Cursor};
use std::path::Path;
use std::net::SocketAddr;
use std::str::FromStr;

use packet::{Mode, RequestPacket, DataPacketOctet, AckPacket, ErrorPacket,
             EncodePacket, RawPacket, Error, Opcode};

use mio::udp::UdpSocket;
use bytes::{MutSliceBuf, MutBuf};

static MAX_DATA_SIZE: usize = 512;

//#[derive(Debug, Clone)]
//pub enum ClientError {
    //TftpError(Error, String),
    //IoError(IoError),
//}

//impl ClientError {
    //pub fn from_io(err: IoError) -> ClientError {
        //ClientError::IoError(err)
    //}
//}

//pub type ClientResult<T> = Result<T, ClientError>;

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
        let mut buf = Cursor::new(encoded.packet_buf());
        self.socket.send_to(&mut buf, &self.remote_addr).map(|_| ())
    }

    fn send_ack(&self, block_id: u16) -> io::Result<()> {
        let ack = AckPacket::new(block_id);
        let encoded = ack.encode();
        let mut buf = Cursor::new(encoded.packet_buf());
        self.socket.send_to(&mut buf, &self.remote_addr).map(|_| ())
    }
}

impl PacketReceiver for InternalClient {
    fn receive_data(&mut self) -> io::Result<DataPacketOctet<'static>> {
        loop {
            let mut buf = vec![0; MAX_DATA_SIZE + 4];
            let (result, n) = {
                let len = buf.len();
                let mut cur = MutSliceBuf::wrap(&mut buf);
                (self.socket.recv_from(&mut cur), len - cur.remaining())
            };
            match result {
                Ok(None) => {
                    continue;
                }
                _ => ()
            }
            return result.map(|from| {
                self.remote_addr = from.expect("no remote address");
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
    pub fn new(remote_addr: SocketAddr) -> io::Result<Client> {
        // FIXME: address should not be hardcoded
        let addr = FromStr::from_str("127.0.0.1:0").unwrap();
        UdpSocket::bound(&addr).map(|socket| {
            Client{ c: InternalClient::new(socket, remote_addr) }
        })
    }

    /// A TFTP read request
    ///
    /// Get a file `path` from the server using a `mode`. Received data is written to
    /// the `writer`.
    pub fn get(&mut self, path: &Path, mode: Mode, writer: &mut io::Write) -> io::Result<()> {
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
                Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "todo"))
            }
        }
        return Ok(())
    }

    /// A TFTP write request
    ///
    /// Put a file `path` to the server using a `mode`.
    pub fn put(&mut self, _path: &Path, _mode: Mode, _reader: &mut io::Read) -> io::Result<()> {
        //let mut bufs = Vec::from_fn(2, |_| Vec::from_elem(MAX_DATA_SIZE + 4, 0));
        //let mut read_buffer = Vec::from_elem(MAX_DATA_SIZE, 0);

        //let read_request = RequestPacket::write_request(path.as_str().expect("utf-8 path"), mode);
        //let encoded = read_request.encode_using(bufs.pop().unwrap());
        //try!(self.socket.send_to(encoded.packet_buf(), self.remote_addr));

        //bufs.push(encoded.get_buffer());
        //let mut first_packet = true;
        //let mut last_packet = false;
        //let mut current_id = 0;
        //loop {
            //let mut buf = bufs.pop().unwrap();
            //match self.socket.recv_from(buf.as_mut_slice()) {
                //Ok((n, from)) => {
                    //if first_packet && self.remote_addr.ip == from.ip {
                        //self.remote_addr.port = from.port;
                        //first_packet = false;
                    //}
                    //if from != self.remote_addr {
                        //bufs.push(buf);
                        //continue
                    //}
                    //let packet = RawPacket::new(buf, n);
                    //{
                        //let ack_packet: Option<AckPacket> = packet.decode();
                        //match ack_packet {
                            //Some(dp) => {
                                //if current_id == dp.block_id() {
                                    //if last_packet {
                                        //println!("done");
                                        //break
                                    //}
                                    //let bytes_read = try!(self.read_block(reader, read_buffer.as_mut_slice()));
                                    //if bytes_read < MAX_DATA_SIZE {
                                        //last_packet = true;
                                    //}
                                    //let ack = DataPacketOctet::from_slice(dp.block_id() + 1, read_buffer.slice_to(bytes_read));
                                    //let buf = bufs.pop().unwrap();
                                    //let encoded = ack.encode_using(buf);
                                    //try!(self.socket.send_to(encoded.packet_buf(), self.remote_addr));
                                    //bufs.push(encoded.get_buffer());
                                    //current_id += 1;
                                //} else {
                                    //println!("wrong packet id");
                                //}
                            //}
                            //None => fail!("not a data packet")
                        //}
                    //}
                    //bufs.push(packet.get_buffer());
                //}
                //Err(ref e) => fail!("error = {}", e)
            //}
        //}
        return Ok(())
    }

    //fn read_block(&mut self, reader: &mut Reader, buf: &mut [u8]) -> IoResult<usize> {
        //reader.read(buf).or_else(|e| {
            //if e.kind == old_io::IoErrorKind::EndOfFile {
                //Ok(0usize)
            //} else {
                //Err(e)
            //}
        //})
    //}
}
