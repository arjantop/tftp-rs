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
use std::str;

use packet::{Mode, RequestPacket, DataPacketOctet, AckPacket, ErrorPacket,
    EncodePacket, RawPacket, Opcode};

use mio::udp::UdpSocket;
use mio::{Events, Poll, PollOpt, Event, Token, Ready};

static MAX_DATA_SIZE: usize = 512;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: io::Error) {
            from()
            description("io error")
            display("I/O error: {}", err)
            cause(err)
        }
        Server(err: ErrorPacket<'static>) {
            from()
            description("server error")
            display("Server error: {}", err)
            cause(err)
        }
    }
}

type Result<T> = result::Result<T, Error>;

trait PacketSender {
    fn send_read_request(&self, path: &str, mode: Mode) -> Result<()>;
    fn send_ack(&self, block_id: u16) -> Result<Option<()>>;
}

trait PacketReceiver {
    fn receive_data(&mut self) -> Result<Option<DataPacketOctet<'static>>>;
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
    fn send_read_request(&self, path: &str, mode: Mode) -> Result<()> {
        let read_request = RequestPacket::read_request(path, mode);
        let encoded = read_request.encode();
        let buf = encoded.packet_buf();
        self.socket.send_to(&buf, &self.remote_addr).map(|_| ()).map_err(From::from)
    }

    fn send_ack(&self, block_id: u16) -> Result<Option<()>> {
        let ack = AckPacket::new(block_id);
        let encoded = ack.encode();
        let buf = encoded.packet_buf();
        self.socket.send_to(&buf, &self.remote_addr).map(|opt| opt.map(|_| ())).map_err(From::from)
    }
}

impl PacketReceiver for InternalClient {
    fn receive_data(&mut self) -> Result<Option<DataPacketOctet<'static>>> {
        let mut buf = vec![0; MAX_DATA_SIZE + 4];
        let result = try!(self.socket.recv_from(&mut buf));
        let p = result.map(|(n, from)| {
            self.remote_addr = from;
            RawPacket::new(buf, n)
        }).map(|packet| {
            match packet.opcode() {
                Some(Opcode::DATA) => {
                    packet.decode::<DataPacketOctet>().unwrap()
                    //                        .ok_or(io::Error::new(io::ErrorKind::Other, "todo")))
                },
                _ => unimplemented!(),
                //                Some(Opcode::ERROR) => return Err(From::from(io::Error::new(io::ErrorKind::Other, "error"))),
                //                _ => return Err(From::from(io::Error::new(io::ErrorKind::Other, "unexpected"))),
            }
        });
        Ok(p)
    }
}

//macro_rules! mtry {
//    ($s: ident, $e:expr) => (match $e {
//        Ok(val) => val,
//        Err(err) => {
//            $s.error = Some(::std::convert::From::from(err));
//            return Client::finish($s);
//        },
//    });
//}


enum ClientStates {
    ReceivingData(u16),
    SendAck(DataPacketOctet<'static>),
    Done,
}

impl ClientStates {
    fn is_done(&self) -> bool {
        match self {
            &ClientStates::Done => true,
            _ => false,
        }
    }
}

struct Client<'a> {
    poll: Poll,
    client: InternalClient,
    path: &'a Path,
    mode: Mode,
    writer: &'a mut io::Write,
}

const CLIENT: Token = Token(0);

impl<'a> Client<'a> {
    fn new(poll: Poll, client: InternalClient, path: &'a Path, mode: Mode, writer: &'a mut io::Write) -> Client<'a> {
        Client {
            poll: poll,
            client: client,
            path: path,
            mode: mode,
            writer: writer,
        }
    }

    //    fn finish(scope: &mut Scope<Context>) -> Response<Self, Void> {
    //        scope.shutdown_loop();
    //        Response::done()
    //    }
}

impl<'a> Client<'a> {
    fn get(&mut self) {
        let mut events = Events::with_capacity(1024);
        let mut current_state = ClientStates::ReceivingData(1);

        self.client.send_read_request(self.path.to_str().unwrap(), Mode::Octet).unwrap();
        println!("Starting transfer ...");
        self.poll.register(&self.client.socket, CLIENT, Ready::readable(), PollOpt::level()).unwrap();

        loop {
            self.poll.poll(&mut events, None).unwrap();
            for event in events.iter() {
                match event.token() {
                    CLIENT => {
                        current_state = self.handle_event(current_state, event);
                        if current_state.is_done() {
                            return;
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn handle_event(&mut self, current_state: ClientStates, event: Event) -> ClientStates {
        match current_state {
            ClientStates::ReceivingData(current_id) => {
//                println!("Receiving data: {}", current_id);
                let data_packet = match self.client.receive_data().unwrap() {
                    Some(data_packet) => data_packet,
                    None => return ClientStates::ReceivingData(current_id),
                };
                if current_id == data_packet.block_id() {
                    self.handle_event(ClientStates::SendAck(data_packet), event)
                } else {
                    println!("Unexpected packet id: got={}, expected={}",
                             data_packet.block_id(), current_id);
                    ClientStates::ReceivingData(current_id)
                }
            }
            ClientStates::SendAck(data_packet) => {
//                println!("Send ack: {}", data_packet.block_id());
                if self.client.send_ack(data_packet.block_id()).unwrap().is_none() {
                    self.poll.reregister(&self.client.socket, CLIENT, Ready::writable(), PollOpt::level()).unwrap();
                    println!("Could not send ack for packet id={}", data_packet.block_id());
                    ClientStates::SendAck(data_packet)
                } else {
                    self.writer.write_all(data_packet.data()).unwrap();
                    if data_packet.data().len() < MAX_DATA_SIZE {
                        println!("Transfer complete");
                        ClientStates::Done
                    } else {
                        if event.kind().is_writable() {
                            self.poll.reregister(&self.client.socket, CLIENT, Ready::readable(), PollOpt::level()).unwrap();
                        }
                        ClientStates::ReceivingData(data_packet.block_id() + 1)
                    }
                }
            }
            _ => unreachable!()
        }
    }
}

pub fn get(path: &Path, mode: Mode, writer: &mut io::Write) {
    println!("starting ...");
    let remote_addr = "127.0.0.1:69".parse().unwrap();
    let any = str::FromStr::from_str("0.0.0.0:0").unwrap();
    let socket = UdpSocket::bind(&any).unwrap();
    let poll =  Poll::new().unwrap();
    let mut client = Client::new(poll, InternalClient::new(socket, remote_addr), path, mode, writer);
    client.get();
}
