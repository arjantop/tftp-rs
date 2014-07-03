extern crate tftp;

use std::io::{File, BufferedReader};
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::os;

use tftp::client::Client;
use tftp::packet::Octet;

fn main() {
    let args = os::args();
    if args.len() != 2 {
        println!("Usage: {} PATH", args.get(0));
        return
    }
    let file_path = args.get(1);
    let file = File::open(&Path::new(file_path.as_slice()));
    let mut reader = BufferedReader::new(file);
    let result = Client::new(SocketAddr{
        ip: Ipv4Addr(127, 0, 0, 1),
        port: 69
    }).and_then(|mut client| {
        client.put(&Path::new("testfile"), Octet, &mut reader)
    });
    if result.is_err() {
        println!("error = {}", result.err().unwrap());
        os::set_exit_status(1);
    }
}
