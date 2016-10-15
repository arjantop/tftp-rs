use std::io::{self, Cursor, Read};
use std::convert::Into;
use std::net::SocketAddr;
use std::thread;

use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;
use tokio_core::channel::{Receiver, channel};
use futures::{Poll, Async};
use futures::stream::Stream;
use futures::Future;

use decodedpacket::DecodedPacket;
use packet::{RequestPacket, RawPacket, DataPacketOctet, EncodePacket, AckPacket};

struct ClientRequest {
    addr: SocketAddr,
    request: DecodedPacket<RequestPacket<'static>>,
}

impl ClientRequest {
    fn new(addr: SocketAddr, request: DecodedPacket<RequestPacket<'static>>) -> ClientRequest {
        ClientRequest {
            addr: addr,
            request: request,
        }
    }
}

struct RequestAcceptor {
    socket: UdpSocket,
}

impl RequestAcceptor {
    fn new(socket: UdpSocket) -> RequestAcceptor {
        RequestAcceptor {
            socket: socket,
        }
    }
}

impl Stream for RequestAcceptor {
    type Item = ClientRequest;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut buf = vec![0; 512];
        let (n, addr) = try_nb!(self.socket.recv_from(&mut buf));

        let packet: DecodedPacket<RequestPacket> = DecodedPacket::decode(RawPacket::new(buf, n)).unwrap();
        Ok(Some(ClientRequest::new(addr, packet)).into())
    }
}

struct RequestHandler {
    socket: UdpSocket,
    client_request: ClientRequest,
    data: Cursor<Vec<u8>>,
    block_id: u16,
    send_data: bool,
    last_id: Option<u16>,
}

impl RequestHandler {
    fn new(socket: UdpSocket, client_request: ClientRequest) -> RequestHandler {
        RequestHandler {
            socket: socket,
            client_request: client_request,
            data: Cursor::new(vec![1; 1025]),
            block_id: 1,
            send_data: true,
            last_id: None,
        }
    }
}

impl Future for RequestHandler {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if self.send_data {
                match self.last_id {
                    Some(last_id) if self.block_id > last_id => break,
                    _ => {}
                }

                let mut buf = vec![0; 512];
                let n = self.data.read(&mut buf).unwrap();

                if n < 512 {
                    self.last_id = Some(self.block_id);
                }

                let data_packet = DataPacketOctet::from_vec(self.block_id, buf, n);
                let encoded_packet = data_packet.encode();

                println!("Sending data packet id = {} length = {}", self.block_id, n);
                println!("{}", encoded_packet.packet_buf().len());
                try_nb!(self.socket.send_to(encoded_packet.packet_buf(), &self.client_request.addr));
                self.send_data = false;
            }

            let mut buf = vec![0; 512];
            let (n, _) = try_nb!(self.socket.recv_from(&mut buf));
            let ack_packet: DecodedPacket<AckPacket> = DecodedPacket::decode(RawPacket::new(buf, n)).unwrap();
            println!("Received ack packet id = {}", ack_packet.block_id());
            self.block_id += 1;
            self.send_data = true;
        }
        Ok(().into())
    }
}

pub fn start() {
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let addr = "127.0.0.1:9999".to_string().parse::<SocketAddr>().unwrap();
    let socket = UdpSocket::bind(&addr, &handle).unwrap();

    println!("Listening on {}", addr);

    let acceptor = RequestAcceptor::new(socket);
    let server = acceptor.for_each(|client_request| {
        println!("mode = {:?}, filename = {:?}", client_request.request.mode(), client_request.request.filename());

        handle.spawn({
            let mut addr = addr.clone();
            addr.set_port(0);
            let socket = UdpSocket::bind(&addr, &handle).unwrap();
            RequestHandler::new(socket, client_request).map_err(|_| ())
        });

        Ok(())
    });

    l.run(server).unwrap();
}
