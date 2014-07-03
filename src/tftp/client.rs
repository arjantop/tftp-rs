//! A Trivial File Transfer (TFTP) protocol client implementation.
//!
//! This module contains the ability to read data from or write data to a remote TFTP server.
use std::io::{IoResult};
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::io::net::udp::UdpSocket;

use packet::{Mode, RequestPacket, DataPacketOctet, AckPacket, EncodePacket, RawPacket};

static MAX_DATA_SIZE: uint = 512;

/// A Trivial File Transfer Protocol client.
pub struct Client {
    socket: UdpSocket,
    remote_addr: SocketAddr,
}

impl Client {
    /// Creates a new client and binds an UDP socket.
    pub fn new(remote_addr: SocketAddr) -> IoResult<Client> {
        // FIXME: port should not be hardcoded
        let addr = SocketAddr{
            ip: Ipv4Addr(127, 0, 0, 1),
            port: 41000
        };
        UdpSocket::bind(addr).map(|socket| {
            Client{ socket: socket, remote_addr: remote_addr }
        })
    }

    /// A TFTP read request
    ///
    /// Get a file `path` from the server using a `mode`. Received data is written to
    /// the `writer`.
    pub fn get(&mut self, path: &Path, mode: Mode, writer: &mut Writer) -> IoResult<()> {
        let mut bufs = Vec::from_fn(2, |_| Vec::from_elem(MAX_DATA_SIZE + 4, 0));

        let read_request = RequestPacket::read_request(path.as_str().expect("utf-8 path"), mode);
        let encoded = read_request.encode_using(bufs.pop().unwrap());
        try!(self.socket.send_to(encoded.packet_buf(), self.remote_addr));

        bufs.push(encoded.get_buffer());
        let mut first_packet = true;
        let mut current_id = 1;
        loop {
            let mut buf = bufs.pop().unwrap();
            match self.socket.recv_from(buf.as_mut_slice()) {
                Ok((n, from)) => {
                    if first_packet && self.remote_addr.ip == from.ip {
                        self.remote_addr.port = from.port;
                        first_packet = false;
                    }
                    if from != self.remote_addr {
                        bufs.push(buf);
                        continue
                    }
                    let packet = RawPacket::new(buf, n);
                    {
                        let data_packet: Option<DataPacketOctet> = packet.decode();
                        match data_packet {
                            Some(dp) => {
                                if current_id == dp.block_id() {
                                    try!(writer.write(dp.data()));
                                    let ack = AckPacket::new(dp.block_id());
                                    let buf = bufs.pop().unwrap();
                                    let encoded = ack.encode_using(buf);
                                    try!(self.socket.send_to(encoded.packet_buf(), self.remote_addr));
                                    if dp.data().len() < MAX_DATA_SIZE {
                                        println!("done");
                                        break;
                                    }
                                    bufs.push(encoded.get_buffer());
                                    current_id += 1;
                                } else {
                                    println!("wrong packet id");
                                }
                            }
                            None => fail!("not a data packet")
                        }
                    }
                    bufs.push(packet.get_buffer());
                }
                Err(ref e) => fail!("error = {}", e)
            }
        }
        return Ok(())
    }

    /// A TFTP write request
    ///
    /// Put a file `path` to the server using a `mode`.
    pub fn put(&mut self, path: &Path, mode: Mode, reader: &mut Reader) -> IoResult<()> {
        let mut bufs = Vec::from_fn(2, |_| Vec::from_elem(MAX_DATA_SIZE + 4, 0));
        let mut read_buffer = Vec::from_elem(MAX_DATA_SIZE, 0);

        let read_request = RequestPacket::write_request(path.as_str().expect("utf-8 path"), mode);
        let encoded = read_request.encode_using(bufs.pop().unwrap());
        try!(self.socket.send_to(encoded.packet_buf(), self.remote_addr));

        bufs.push(encoded.get_buffer());
        let mut first_packet = true;
        let mut last_packet = false;
        let mut current_id = 0;
        loop {
            let mut buf = bufs.pop().unwrap();
            match self.socket.recv_from(buf.as_mut_slice()) {
                Ok((n, from)) => {
                    if first_packet && self.remote_addr.ip == from.ip {
                        self.remote_addr.port = from.port;
                        first_packet = false;
                    }
                    if from != self.remote_addr {
                        bufs.push(buf);
                        continue
                    }
                    let packet = RawPacket::new(buf, n);
                    {
                        let ack_packet: Option<AckPacket> = packet.decode();
                        match ack_packet {
                            Some(dp) => {
                                if current_id == dp.block_id() {
                                    if last_packet {
                                        println!("done");
                                        break
                                    }
                                    let bytes_read = try!(reader.read(read_buffer.as_mut_slice()));
                                    if bytes_read < MAX_DATA_SIZE {
                                        last_packet = true;
                                    }
                                    let ack = DataPacketOctet::from_slice(dp.block_id() + 1, read_buffer.slice_to(bytes_read));
                                    let buf = bufs.pop().unwrap();
                                    let encoded = ack.encode_using(buf);
                                    try!(self.socket.send_to(encoded.packet_buf(), self.remote_addr));
                                    bufs.push(encoded.get_buffer());
                                    current_id += 1;
                                } else {
                                    println!("wrong packet id");
                                }
                            }
                            None => fail!("not a data packet")
                        }
                    }
                    bufs.push(packet.get_buffer());
                }
                Err(ref e) => fail!("error = {}", e)
            }
        }
        return Ok(())
    }
}
